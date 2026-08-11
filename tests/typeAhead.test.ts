import { describe, expect, test } from "bun:test";
import { typeAheadMatch } from "../frontend/src/lib/typeAhead";

const names = ["Documents", "Downloads", "Music", "downloads.txt", "notes.md"];

describe("typeAheadMatch", () => {
	test("a prefix finds the first matching entry", () => {
		expect(typeAheadMatch(names, "mu", -1)).toBe(2);
	});

	test("matching ignores case", () => {
		expect(typeAheadMatch(names, "docu", -1)).toBe(0);
	});

	test("repeating one character cycles through the matches", () => {
		expect(typeAheadMatch(names, "d", -1)).toBe(0);
		expect(typeAheadMatch(names, "d", 0)).toBe(1);
		expect(typeAheadMatch(names, "d", 1)).toBe(3);
		expect(typeAheadMatch(names, "d", 3)).toBe(0);
	});

	test("a longer query stays on the current entry when it still matches", () => {
		expect(typeAheadMatch(names, "downloads", 1)).toBe(1);
	});

	test("no match reports nothing to jump to", () => {
		expect(typeAheadMatch(names, "zz", -1)).toBe(-1);
		expect(typeAheadMatch([], "d", -1)).toBe(-1);
	});
});
