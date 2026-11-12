import { describe, expect, test } from "bun:test";
import { nameMatches, nextMatchIndex } from "../ui/lib/finder";

const names = ["Documents", "Downloads", "Music", "downloads.txt", "notes.md"];

describe("nameMatches", () => {
	test("an empty query matches everything", () => {
		expect(nameMatches("anything", "")).toBe(true);
	});

	test("a lower case query matches anywhere in the name, ignoring case", () => {
		expect(nameMatches("My Documents", "doc")).toBe(true);
		expect(nameMatches("My Documents", "ments")).toBe(true);
		expect(nameMatches("My Documents", "xyz")).toBe(false);
	});

	test("a query with a capital letter is exact about case", () => {
		expect(nameMatches("My documents", "Doc")).toBe(false);
		expect(nameMatches("My Documents", "Doc")).toBe(true);
	});
});

describe("nextMatchIndex", () => {
	test("finds the first match after the cursor", () => {
		expect(nextMatchIndex(names, "down", 0, "forward", false)).toBe(1);
		expect(nextMatchIndex(names, "down", 1, "forward", false)).toBe(3);
	});

	test("wraps around past the last entry", () => {
		expect(nextMatchIndex(names, "down", 3, "forward", false)).toBe(1);
	});

	test("searches backwards and wraps past the first entry", () => {
		expect(nextMatchIndex(names, "down", 3, "backward", false)).toBe(1);
		expect(nextMatchIndex(names, "down", 1, "backward", false)).toBe(3);
	});

	test("a query still being typed stays on the entry it already matches", () => {
		expect(nextMatchIndex(names, "downloads", 1, "forward", true)).toBe(1);
	});

	test("starts at the first entry when there is no cursor yet", () => {
		expect(nextMatchIndex(names, "d", -1, "forward", false)).toBe(0);
	});

	test("reports no match for an unmatched query or an empty list", () => {
		expect(nextMatchIndex(names, "zz", 0, "forward", false)).toBe(-1);
		expect(nextMatchIndex([], "d", -1, "forward", false)).toBe(-1);
		expect(nextMatchIndex(names, "", 0, "forward", false)).toBe(-1);
	});
});
