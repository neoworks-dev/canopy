import { describe, expect, test } from "bun:test";
import {
	defaultSortOrder,
	sortDirectionOf,
	sortOrderAfterHeaderClick,
	sortOrderWithDirectoriesFirst,
} from "../frontend/src/lib/sorting";

describe("sortOrderAfterHeaderClick", () => {
	test("a new column sorts ascending", () => {
		const order = sortOrderAfterHeaderClick(defaultSortOrder, "size");
		expect(order.key).toBe("size");
		expect(order.descending).toBe(false);
	});

	test("clicking the active column reverses it", () => {
		const ascending = sortOrderAfterHeaderClick(defaultSortOrder, "size");
		expect(sortOrderAfterHeaderClick(ascending, "size").descending).toBe(true);
	});

	test("a third click sorts ascending again", () => {
		const ascending = sortOrderAfterHeaderClick(defaultSortOrder, "size");
		const descending = sortOrderAfterHeaderClick(ascending, "size");
		expect(sortOrderAfterHeaderClick(descending, "size").descending).toBe(false);
	});

	test("moving to another column drops the reversed direction", () => {
		const descending = { key: "size", descending: true, directoriesFirst: true } as const;
		expect(sortOrderAfterHeaderClick(descending, "modified").descending).toBe(false);
	});

	test("the folders-first choice survives a header click", () => {
		const mixed = sortOrderWithDirectoriesFirst(defaultSortOrder, false);
		expect(sortOrderAfterHeaderClick(mixed, "modified").directoriesFirst).toBe(false);
	});
});

describe("sortOrderWithDirectoriesFirst", () => {
	test("the key and direction are left alone", () => {
		const descending = sortOrderAfterHeaderClick(
			sortOrderAfterHeaderClick(defaultSortOrder, "size"),
			"size",
		);
		const mixed = sortOrderWithDirectoriesFirst(descending, false);
		expect(mixed.key).toBe("size");
		expect(mixed.descending).toBe(true);
	});
});

describe("sortDirectionOf", () => {
	test("the active column reports its direction", () => {
		expect(sortDirectionOf(defaultSortOrder, "name")).toBe("ascending");
	});

	test("a reversed active column reports descending", () => {
		const descending = sortOrderAfterHeaderClick(defaultSortOrder, "name");
		expect(sortDirectionOf(descending, "name")).toBe("descending");
	});

	test("every other column reports nothing", () => {
		expect(sortDirectionOf(defaultSortOrder, "size")).toBe("");
	});
});
