import { unlink } from "node:fs/promises";
import { connect, createServer, type Server, type Socket } from "node:net";

export type Verbs = Record<
	string,
	(payload: unknown) => unknown | Promise<unknown>
>;

export type Served = {
	close: () => Promise<void>;
};

export type Control = {
	port: () => number | undefined;
};

export type Inspect = {
	serve: (verbs: Verbs) => Promise<Served>;
};

export type Held = {
	control: Control;
	inspect?: Inspect;
};

export const client = {
	connect: async (held: NodeJS.ProcessEnv = process.env): Promise<Held> => {
		const seat = read(held.SIDECAR_INSPECT);
		const port = read(held.SIDECAR_PORT);
		return {
			control: { port: () => decimal(port) },
			inspect: seat === undefined ? undefined : { serve: serve(seat) },
		};
	},
};

function read(raw: string | undefined): string | undefined {
	if (raw === undefined) return undefined;
	if (raw.length === 0) {
		throw new Error(
			"a sidecar word reads empty; absence is stated by not setting it",
		);
	}
	return raw;
}

function decimal(raw: string | undefined): number | undefined {
	if (raw === undefined) return undefined;
	if (!/^\d+$/.test(raw)) {
		throw new Error(`SIDECAR_PORT must read decimal but reads ${raw}`);
	}
	return Number(raw);
}

function serve(seat: string): (verbs: Verbs) => Promise<Served> {
	return async (verbs: Verbs): Promise<Served> => {
		await clear(seat);
		const server = createServer((socket) => {
			answer(socket, verbs);
		});
		await listen(server, seat);
		return {
			close: () =>
				new Promise<void>((settle, refuse) => {
					server.close((error) => (error ? refuse(error) : settle()));
				}),
		};
	};
}

async function clear(seat: string): Promise<void> {
	const live = await new Promise<boolean>((settle) => {
		const probe = connect(seat);
		probe.on("connect", () => {
			probe.destroy();
			settle(true);
		});
		probe.on("error", () => {
			probe.destroy();
			settle(false);
		});
	});
	if (live) {
		throw new Error(
			`${seat} already answers; sidecar owns one seat per target`,
		);
	}
	await unlink(seat).catch(() => undefined);
}

function listen(server: Server, seat: string): Promise<void> {
	return new Promise<void>((settle, refuse) => {
		server.once("error", refuse);
		server.listen(seat, () => {
			server.removeListener("error", refuse);
			settle();
		});
	});
}

function answer(socket: Socket, verbs: Verbs): void {
	let held = "";
	socket.setEncoding("utf8");
	socket.on("data", (chunk: string) => {
		held += chunk;
		const cut = held.indexOf("\n");
		if (cut < 0) return;
		const line = held.slice(0, cut);
		held = held.slice(cut + 1);
		void reply(socket, verbs, line);
	});
	socket.on("error", () => socket.destroy());
}

async function reply(
	socket: Socket,
	verbs: Verbs,
	line: string,
): Promise<void> {
	const frame = decode(line);
	if (frame === undefined) {
		socket.end();
		return;
	}
	const held = verbs[frame.verb];
	if (held === undefined) {
		socket.write(
			`${JSON.stringify({
				kind: "event_error",
				id: frame.id,
				error: { code: "unknown_verb", message: `no verb ${frame.verb}` },
			})}\n`,
		);
		return;
	}
	try {
		const payload = await held(frame.payload);
		socket.write(
			`${JSON.stringify({
				kind: "event_response",
				id: frame.id,
				payload: payload ?? {},
			})}\n`,
		);
	} catch (error) {
		socket.write(
			`${JSON.stringify({
				kind: "event_error",
				id: frame.id,
				error: { code: "verb_failed", message: String(error) },
			})}\n`,
		);
	}
}

export type Frame = {
	id: string;
	verb: string;
	payload: unknown;
};

export function decode(line: string): Frame | undefined {
	const trimmed = line.trim();
	if (trimmed.length === 0) return undefined;
	let held: unknown;
	try {
		held = JSON.parse(trimmed);
	} catch {
		return undefined;
	}
	if (typeof held !== "object" || held === null) return undefined;
	const frame = held as Record<string, unknown>;
	if (frame.kind !== "event") return undefined;
	if (typeof frame.id !== "string" || typeof frame.verb !== "string") {
		return undefined;
	}
	return { id: frame.id, verb: frame.verb, payload: frame.payload ?? {} };
}
