import { describe, expect, test } from "bun:test";
import { evaluateWhen, whenClauseIsValid } from "../frontend/src/lib/whenClause";

const context = {
	listFocused: true,
	searchOpen: false,
	selectionCount: 2,
	clipboardCount: 0,
	viewMode: "grid",
	mode: "normal",
	emptyLabel: "",
};

describe("evaluateWhen", () => {
	test("an empty clause always applies", () => {
		expect(evaluateWhen("", context)).toBe(true);
		expect(evaluateWhen("   ", context)).toBe(true);
	});

	test("a bare identifier reads its context value", () => {
		expect(evaluateWhen("listFocused", context)).toBe(true);
		expect(evaluateWhen("searchOpen", context)).toBe(false);
	});

	test("an identifier the context does not define is false", () => {
		expect(evaluateWhen("trashOpen", context)).toBe(false);
	});

	test("an empty string is not truthy", () => {
		expect(evaluateWhen("emptyLabel", context)).toBe(false);
	});

	test("a number is truthy unless it is zero", () => {
		expect(evaluateWhen("selectionCount", context)).toBe(true);
		expect(evaluateWhen("clipboardCount", context)).toBe(false);
	});
});

describe("operators", () => {
	test("negation", () => {
		expect(evaluateWhen("!searchOpen", context)).toBe(true);
		expect(evaluateWhen("!listFocused", context)).toBe(false);
	});

	test("conjunction and disjunction", () => {
		expect(evaluateWhen("listFocused && !searchOpen", context)).toBe(true);
		expect(evaluateWhen("listFocused && searchOpen", context)).toBe(false);
		expect(evaluateWhen("searchOpen || listFocused", context)).toBe(true);
	});

	test("parentheses group against the default precedence", () => {
		expect(evaluateWhen("searchOpen && (listFocused || searchOpen)", context)).toBe(false);
		expect(evaluateWhen("(searchOpen && listFocused) || listFocused", context)).toBe(true);
	});

	test("and binds tighter than or", () => {
		expect(evaluateWhen("searchOpen && searchOpen || listFocused", context)).toBe(true);
	});

	test("numeric comparison", () => {
		expect(evaluateWhen("selectionCount > 1", context)).toBe(true);
		expect(evaluateWhen("selectionCount > 2", context)).toBe(false);
		expect(evaluateWhen("selectionCount >= 2", context)).toBe(true);
		expect(evaluateWhen("clipboardCount < 1", context)).toBe(true);
		expect(evaluateWhen("selectionCount <= 1", context)).toBe(false);
	});

	test("equality", () => {
		expect(evaluateWhen("selectionCount == 2", context)).toBe(true);
		expect(evaluateWhen("selectionCount != 2", context)).toBe(false);
	});

	// So a clause reads as `viewMode == grid` rather than needing quotes.
	test("an undefined identifier on the right compares as its own name", () => {
		expect(evaluateWhen("viewMode == grid", context)).toBe(true);
		expect(evaluateWhen("viewMode == rows", context)).toBe(false);
	});

	test("quoted values work too", () => {
		expect(evaluateWhen("viewMode == 'grid'", context)).toBe(true);
		expect(evaluateWhen('mode == "normal"', context)).toBe(true);
	});

	test("comparing against a defined identifier uses its value", () => {
		expect(evaluateWhen("selectionCount == selectionCount", context)).toBe(true);
	});

	test("an unorderable comparison is false rather than an error", () => {
		expect(evaluateWhen("viewMode > 1", context)).toBe(false);
	});
});

describe("malformed clauses", () => {
	// A clause that cannot be read must not enable the binding it guards.
	test("a clause that does not parse is false", () => {
		expect(evaluateWhen("listFocused &&", context)).toBe(false);
		expect(evaluateWhen("(listFocused", context)).toBe(false);
		expect(evaluateWhen("&& listFocused", context)).toBe(false);
		expect(evaluateWhen("listFocused $$ searchOpen", context)).toBe(false);
	});

	test("validity is reportable on its own, for checking stored settings", () => {
		expect(whenClauseIsValid("listFocused && selectionCount > 0")).toBe(true);
		expect(whenClauseIsValid("")).toBe(true);
		expect(whenClauseIsValid("listFocused &&")).toBe(false);
	});

	test("a repeated clause gives the same answer, so caching is not a trap", () => {
		expect(evaluateWhen("listFocused", context)).toBe(true);
		expect(evaluateWhen("listFocused", { listFocused: false })).toBe(false);
	});
});
