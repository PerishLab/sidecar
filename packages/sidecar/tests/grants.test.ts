import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { grants } from "../src/lib.ts";

type Spec = { term: string; form: string };
type Case = {
	name: string;
	environment: Record<string, string>;
	expects: Record<string, string | null>;
};

const root = join(dirname(fileURLToPath(import.meta.url)), "../../..");

function stated(name: string): Record<string, never> {
	const text = readFileSync(join(root, name), "utf8");
	const bare = text
		.split("\n")
		.filter((line) => !line.trimStart().startsWith("//"))
		.join("\n");
	return JSON.parse(bare);
}

function words(): Record<string, Spec> {
	return stated("sidecar.schema.jsonc").words;
}

function render(form: string, raw: string): string | number {
	return form === "decimal" ? Number(raw) : raw;
}

function nest(term: string, value: string | number): Record<string, unknown> {
	const parts = term.split("_").reverse();
	let held: unknown = value;
	for (const part of parts) held = { [part]: held };
	return held as Record<string, unknown>;
}

function merge(
	into: Record<string, unknown>,
	from: Record<string, unknown>,
): void {
	for (const [key, value] of Object.entries(from)) {
		const seat = into[key];
		if (seat && typeof seat === "object" && typeof value === "object") {
			merge(seat as Record<string, unknown>, value as Record<string, unknown>);
			continue;
		}
		into[key] = value;
	}
}

test("every declared word is read, nested by its term and typed by its form", () => {
	const held: Record<string, string> = {};
	const want: Record<string, unknown> = {};
	for (const [word, spec] of Object.entries(words())) {
		const raw = spec.form === "decimal" ? "4287" : "seat";
		held[word] = raw;
		merge(want, nest(spec.term, render(spec.form, raw)));
	}
	expect(grants(held)).toEqual(want);
});

test("the fixture cases conform", () => {
	const forms: Record<string, string> = {};
	for (const spec of Object.values(words())) forms[spec.term] = spec.form;
	const cases: Case[] = stated("sidecar.fixture.jsonc").cases;
	expect(cases.length).toBeGreaterThan(0);
	for (const one of cases) {
		const want: Record<string, unknown> = {};
		for (const [term, value] of Object.entries(one.expects)) {
			if (value !== null) merge(want, nest(term, render(forms[term], value)));
		}
		expect(grants(one.environment), one.name).toEqual(want);
	}
});
