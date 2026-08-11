import { describe, expect, test } from "bun:test";
import {
	emptyTypeAhead,
	isTypeAheadKey,
	typeAheadAfterKey,
	typeAheadMatch,
	typeAheadResetMilliseconds,
} from "../frontend/src/lib/typeAhead";

const names = ["Documents", "Downloads", "Music", "downloads.txt", "notes.md"];

function key(character: string, modifiers: Partial<Record<"ctrlKey" | "metaKey" | "altKey", boolean>> = {}) {
	return { key: character, ctrlKey: false, metaKey: false, altKey: false, ...modifiers };
}

describe("isTypeAheadKey", () => {
	test("a bare printable character starts a search", () => {
		expect(isTypeAheadKey(key("d"))).toBe(true);
	});

	test("a shortcut is not type-ahead", () => {
		expect(isTypeAheadKey(key("a", { ctrlKey: true }))).toBe(false);
		expect(isTypeAheadKey(key("l", { metaKey: true }))).toBe(false);
		expect(isTypeAheadKey(key("ArrowLeft", { altKey: true }))).toBe(false);
	});

	test("named keys and space are not type-ahead", () => {
		expect(isTypeAheadKey(key("Enter"))).toBe(false);
		expect(isTypeAheadKey(key("ArrowDown"))).toBe(false);
		expect(isTypeAheadKey(key(" "))).toBe(false);
	});
});

describe("typeAheadAfterKey", () => {
	test("consecutive keystrokes build one query", () => {
		const first = typeAheadAfterKey(emptyTypeAhead, "D", 1000);
		const second = typeAheadAfterKey(first, "o", 1100);
		expect(second.query).toBe("do");
	});

	test("a pause starts a new query", () => {
		const first = typeAheadAfterKey(emptyTypeAhead, "d", 1000);
		const later = typeAheadAfterKey(first, "m", 1000 + typeAheadResetMilliseconds + 1);
		expect(later.query).toBe("m");
	});
});

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
