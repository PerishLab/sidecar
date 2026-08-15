import { expect, test } from "vitest";
import { client, decode } from "../src/lib.ts";

test("a declared port reads decimal", async () => {
	const held = await client.connect({ SIDECAR_PORT: "44767" });
	expect(held.control.port()).toBe(44767);
});

test("an absent word is absent, not zero", async () => {
	const held = await client.connect({});
	expect(held.control.port()).toBeUndefined();
});

test("an absent grant makes the facet absent", async () => {
	const held = await client.connect({});
	expect(held.inspect).toBeUndefined();
});

test("a declared grant makes the facet present", async () => {
	const held = await client.connect({ SIDECAR_INSPECT: "/tmp/held.sock" });
	expect(held.inspect).toBeDefined();
});

test("an empty word refuses rather than reading as absent", async () => {
	await expect(client.connect({ SIDECAR_PORT: "" })).rejects.toThrow();
});

test("a port that is not decimal refuses", async () => {
	const held = await client.connect({ SIDECAR_PORT: "4a" });
	expect(() => held.control.port()).toThrow();
});

test("an event frame decodes to id, verb and payload", () => {
	const frame = decode(
		'{"kind":"event","id":"1","verb":"ready","payload":{"a":1}}',
	);
	expect(frame).toEqual({ id: "1", verb: "ready", payload: { a: 1 } });
});

test("a missing payload decodes as the unit shape", () => {
	const frame = decode('{"kind":"event","id":"1","verb":"ready"}');
	expect(frame?.payload).toEqual({});
});

test("a frame that is not an event decodes to nothing", () => {
	expect(decode('{"kind":"hello","id":"1","verb":"ready"}')).toBeUndefined();
	expect(decode("not json")).toBeUndefined();
	expect(decode("")).toBeUndefined();
});
