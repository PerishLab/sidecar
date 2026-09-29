import { spawn } from "node:child_process";
import {
	closeSync,
	mkdirSync,
	openSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

const namespace = `sc-${process.pid}`;
const seat = join(tmpdir(), namespace);
const home = join(seat, "home");
const config = join(seat, "sidecar.toml");
const script = resolve("tests", "probe.ts");

type Ran = {
	code: number | null;
	out: string;
	err: string;
};

function manifest(): string {
	return [
		"[project]",
		'name = "probe"',
		`namespace = "${namespace}"`,
		'root = "."',
		"",
		"[[sidecars]]",
		'name = "probe"',
		"command = 'node'",
		`args = ['--conditions=source', '${script}']`,
		'cwd = "."',
		'mode = "probe"',
		"port = 0",
		"inspect = {}",
		"",
	].join("\n");
}

function sidecar(verbs: string[], sink?: number): Promise<Ran> {
	const flags = ["--config", config, "--data-home", home];
	const cargo = ["run", "--quiet", "--locked", "--bin", "sidecar", "--"];
	const child = spawn("cargo", [...cargo, ...flags, ...verbs], {
		stdio: ["ignore", sink ?? "pipe", sink ?? "pipe"],
	});
	let out = "";
	let err = "";
	child.stdout?.on("data", (chunk: Buffer) => {
		out += chunk.toString();
	});
	child.stderr?.on("data", (chunk: Buffer) => {
		err += chunk.toString();
	});
	return new Promise<Ran>((settle, refuse) => {
		child.once("error", refuse);
		child.once("close", (code) => settle({ code, out, err }));
	});
}

async function settle(): Promise<unknown> {
	const deadline = Date.now() + 30_000;
	let last = "inspect was never attempted";
	while (Date.now() < deadline) {
		const ran = await sidecar([
			"inspect",
			"probe",
			"server.status",
			"--format=json",
		]);
		if (ran.code === 0) return JSON.parse(ran.out);
		last = ran.err.trim();
		await new Promise((wake) => setTimeout(wake, 200));
	}
	throw new Error(`the probe never answered inspect: ${last}`);
}

function said(): string {
	const log = join(home, "projects", namespace, "logs", "probe.log");
	try {
		return readFileSync(log, "utf8").trim();
	} catch (error) {
		return `the target log is unreadable: ${String(error)}`;
	}
}

test("the binding answers inspect through this tree's sidecar", async () => {
	rmSync(seat, { recursive: true, force: true });
	mkdirSync(seat, { recursive: true });
	writeFileSync(config, manifest());
	const trace = join(seat, "start.log");
	const sink = openSync(trace, "w");
	const start = await sidecar(["start"], sink);
	closeSync(sink);
	expect(start.code, readFileSync(trace, "utf8")).toBe(0);
	try {
		const answer = (await settle().catch((error) => {
			throw new Error(`${String(error)}\nthe target said:\n${said()}`);
		})) as { ok?: boolean; data?: { port?: number } };
		expect(answer.ok).toBe(true);
		expect(answer.data?.port).toBeGreaterThan(0);
	} finally {
		await sidecar(["reset", "--force"]);
		rmSync(seat, { recursive: true, force: true });
	}
}, 600_000);
