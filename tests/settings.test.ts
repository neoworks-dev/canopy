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

	test("stored pane widths are applied, out-of-range ones clamped", () => {
		const applied = viewSettingsFrom({ panes: { sidebar: 300, details: 5000 } });
		expect(applied.panes).toEqual({ sidebar: 300, details: 640 });
	});

	test("a file without pane widths keeps the defaults", () => {
		expect(viewSettingsFrom({ panes: {} }).panes).toEqual(defaultViewSettings.panes);
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

describe("stored keybindings", () => {
	test("a file without a keymap contributes none", () => {
		expect(viewSettingsFrom({}).keybindings).toEqual([]);
	});

	test("a binding is read with its clause", () => {
		expect(
			viewSettingsFrom({
				keybindings: [{ key: "g g", command: "list.move.first", when: "listActive" }],
			}).keybindings,
		).toEqual([{ key: "g g", command: "list.move.first", when: "listActive" }]);
	});

	test("a missing clause becomes an unconditional binding", () => {
		expect(viewSettingsFrom({ keybindings: [{ key: "f", command: "search.here" }] }).keybindings)
			.toEqual([{ key: "f", command: "search.here", when: "" }]);
	});

	// An empty command is how a line unbinds a built-in key.
	test("an empty command is kept", () => {
		expect(viewSettingsFrom({ keybindings: [{ key: "f", command: "" }] }).keybindings).toEqual([
			{ key: "f", command: "", when: "" },
		]);
	});

	// One bad line in a hand-edited file must not cost the others.
	test("a line that cannot be read is dropped on its own", () => {
		const kept = viewSettingsFrom({
			keybindings: [
				{ key: "hyper+q", command: "search.here" },
				{ key: "f", command: "search.here" },
				{ key: "z", command: "search.global", when: "listActive &&" },
				{ command: "search.here" },
				{ key: "x" },
			],
		}).keybindings;
		expect(kept.map((binding) => binding.key)).toEqual(["f"]);
	});

	test("a keymap of the wrong shape is ignored", () => {
		expect(viewSettingsFrom({ keybindings: undefined }).keybindings).toEqual([]);
	});
});
