import { describe, expect, test } from "bun:test";
import {
	isNameTaken,
	searchDescription,
	suggestedName,
	type SavedSearch,
} from "../frontend/src/lib/savedSearches";

function search(name: string, query: string, scope = ""): SavedSearch {
	return { name, query, scope } as SavedSearch;
}

describe("the name a search is offered under", () => {
	test("a search of everywhere is named after the query", () => {
		expect(suggestedName("invoice", "")).toBe("invoice");
	});

	test("a scoped search says which folder it looked in", () => {
		expect(suggestedName("invoice", "/home/user/documents")).toBe("invoice in documents");
	});

	test("a search scoped to the root names the root", () => {
		expect(suggestedName("invoice", "/")).toBe("invoice in /");
	});

	test("a trailing separator does not become an empty folder name", () => {
		expect(suggestedName("invoice", "/home/user/documents/")).toBe("invoice in documents");
	});

	test("the query is trimmed, since it is typed", () => {
		expect(suggestedName("  invoice  ", "")).toBe("invoice");
	});

	test("an empty query has nothing to be named after", () => {
		expect(suggestedName("   ", "/home/user")).toBe("");
	});
});

describe("what the sidebar says about a saved search", () => {
	test("an unscoped search says it looks everywhere", () => {
		expect(searchDescription(search("Invoices", "invoice"))).toBe("invoice · everywhere");
	});

	test("a scoped search says where it looks", () => {
		expect(searchDescription(search("Invoices", "invoice", "/home/user/docs"))).toBe(
			"invoice · in /home/user/docs",
		);
	});
});

describe("whether a name is free", () => {
	const saved = [search("Invoices", "invoice"), search("Photos", "jpg")];

	test("a name already saved is taken", () => {
		expect(isNameTaken(saved, "Photos")).toBe(true);
	});

	test("a name nothing is saved under is free", () => {
		expect(isNameTaken(saved, "Receipts")).toBe(false);
	});

	test("the name is compared trimmed, which is how it is saved", () => {
		expect(isNameTaken(saved, "  Photos  ")).toBe(true);
	});
});
