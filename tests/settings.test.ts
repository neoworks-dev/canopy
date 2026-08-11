import { describe, expect, test } from "bun:test";
import { defaultViewSettings, viewSettingsFrom } from "../frontend/src/lib/settings";

describe("viewSettingsFrom", () => {
	test("an empty file gives the defaults", () => {
		expect(viewSettingsFrom({})).toEqual(defaultViewSettings);
	});

	test("stored values are applied", () => {
		const applied = viewSettingsFrom({
			viewMode: "grid",
			columns: ["kind", "permissions"],
			showHidden: true,
			sort: { key: "size", descending: true, directoriesFirst: false },
			lastDirectory: "/home/moritz",
		});
		expect(applied.viewMode).toBe("grid");
		expect(applied.columns).toEqual(["kind", "permissions"]);
		expect(applied.showHidden).toBe(true);
		expect(applied.sort).toEqual({ key: "size", descending: true, directoriesFirst: false });
		expect(applied.lastDirectory).toBe("/home/moritz");
	});

	test("an unknown view mode falls back to rows", () => {
		expect(viewSettingsFrom({ viewMode: "columns" }).viewMode).toBe("rows");
	});

	test("column ids that no longer exist are dropped", () => {
		expect(viewSettingsFrom({ columns: ["size", "colour", "modified"] }).columns).toEqual([
			"size",
			"modified",
		]);
	});

	test("a file that turned every column off keeps them off", () => {
		expect(viewSettingsFrom({ columns: [] }).columns).toEqual([]);
	});

	test("an unknown sort key falls back to the name", () => {
		expect(viewSettingsFrom({ sort: { key: "colour" } }).sort.key).toBe("name");
	});

	test("folders-first stays on unless the file turned it off", () => {
		expect(viewSettingsFrom({ sort: { key: "size" } }).sort.directoriesFirst).toBe(true);
		expect(viewSettingsFrom({ sort: { directoriesFirst: false } }).sort.directoriesFirst).toBe(
			false,
		);
	});

	test("a last directory of the wrong type is ignored", () => {
		expect(viewSettingsFrom({ lastDirectory: undefined }).lastDirectory).toBe("");
	});
});
