import { describe, expect, test } from "bun:test";
import { continuationRows, filterHelpRows, helpRows } from "../ui/lib/help";
import { compileKeymap } from "../ui/lib/keymap";

describe("the help panel", () => {
	const keymap = compileKeymap([
		{ key: "g h", command: "navigate.home", when: "listActive" },
		{ key: "q", command: "app.quit", when: "listActive" },
		{ key: "q", command: "", when: "listActive" },
		{ key: "d", command: "file.trash", when: "listActive" },
		{ key: "d", command: "file.deletePermanently", when: "listActive" },
	]);
	const titles: Record<string, string> = {
		"navigate.home": "Go to home folder",
		"file.deletePermanently": "Delete permanently",
	};
	const rows = helpRows(keymap, (command) => titles[command]);

	test("unbound and overridden keys are not listed", () => {
		expect(rows.map((row) => `${row.keys}=${row.command}`)).toEqual([
			"G H=navigate.home",
			"D=file.deletePermanently",
		]);
	});

	test("which-key lists the next key of each sequence that still applies", () => {
		const sequences = compileKeymap([
			{ key: "g h", command: "navigate.home", when: "" },
			{ key: "g f", command: "navigate.followLink", when: "hidden" },
			{ key: "g", command: "unrelated", when: "" },
			{ key: "c c", command: "copy.path", when: "" },
		]);
		const rows = continuationRows(
			sequences,
			["g"],
			(binding) => binding.when !== "hidden",
			(command) => command,
		);
		expect(rows.map((row) => `${row.keys}=${row.command}`)).toEqual(["H=navigate.home"]);
	});

	test("the filter matches keys or titles, ignoring case", () => {
		expect(filterHelpRows(rows, "HOME").map((row) => row.command)).toEqual(["navigate.home"]);
		expect(filterHelpRows(rows, "g h").map((row) => row.command)).toEqual(["navigate.home"]);
		expect(filterHelpRows(rows, " ")).toHaveLength(2);
	});
});
