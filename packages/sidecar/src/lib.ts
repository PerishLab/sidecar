export type Grants = {
	port?: number;
	inspectSocket?: string;
	broker?: string;
};

export type Held = Record<string, string | undefined>;

export function grants(held: Held): Grants {
	const said: Grants = {};
	const port = decimal(held.SIDECAR_PORT);
	if (port !== undefined) said.port = port;
	const socket = endpoint(held.SIDECAR_INSPECT_SOCKET);
	if (socket !== undefined) said.inspectSocket = socket;
	const broker = endpoint(held.SIDECAR_BROKER);
	if (broker !== undefined) said.broker = broker;
	return said;
}

function decimal(raw: string | undefined): number | undefined {
	if (raw === undefined) return undefined;
	if (!/^\d+$/.test(raw)) {
		throw new Error(
			`SIDECAR_PORT is declared decimal but reads ${JSON.stringify(raw)}`,
		);
	}
	return Number(raw);
}

function endpoint(raw: string | undefined): string | undefined {
	if (raw === undefined) return undefined;
	if (raw.length === 0) {
		throw new Error(
			"an endpoint word reads empty; absence is stated by not setting it",
		);
	}
	return raw;
}
