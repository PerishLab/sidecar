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

function idiom(term: string): string {
	return term
		.split("_")
		.map((part, index) =>
			index === 0 ? part : part[0].toUpperCase() + part.slice(1),
		)
		.join("");
}

function render(form: string, raw: string): string | number {
	return form === "decimal" ? Number(raw) : raw;
}

function wanted(
	one: Case,
	forms: Record<string, string>,
): Record<string, unknown> {
	const want: Record<string, unknown> = {};
	for (const [term, value] of Object.entries(one.expects)) {
		if (value !== null) want[idiom(term)] = render(forms[term], value);
	}
	return want;
}

test("every declared word is read and rendered by its form", () => {
	const held: Record<string, string> = {};
	const want: Record<string, unknown> = {};
	for (const [word, spec] of Object.entries(words())) {
		const raw = spec.form === "decimal" ? "4287" : "seat";
		held[word] = raw;
		want[idiom(spec.term)] = render(spec.form, raw);
	}
	expect(grants(held)).toEqual(want);
});

test("the fixture cases conform", () => {
	const forms: Record<string, string> = {};
	for (const spec of Object.values(words())) forms[spec.term] = spec.form;
	const cases: Case[] = stated("sidecar.fixture.jsonc").cases;
	expect(cases.length).toBeGreaterThan(0);
	for (const one of cases) {
		expect(grants(one.environment), one.name).toEqual(wanted(one, forms));
	}
});
