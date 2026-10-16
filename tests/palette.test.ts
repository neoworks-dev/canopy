import { describe, expect, test } from "bun:test";
import type { Command } from "../ui/lib/commands";
import { availableCommands, rankCommands, titleSegments } from "../ui/lib/palette";

function command(id: string, title: string, category: string, when?: string): Command {
	return { id, title, category, when, run: () => {} };
}

const commands = [
	command("navigate.terminal", "Open terminal here", "Navigation"),
	command("file.rename", "Rename", "File", "selectionCount == 1"),
	command("file.newFolder", "New folder", "File"),
	command("view.grid", "Grid view", "View"),
];

function titlesFor(query: string): string[] {
	return rankCommands(commands, query).map((match) => match.command.title);
}

describe("the commands a palette offers", () => {
	test("a command whose clause does not hold is left out", () => {
		const offered = availableCommands(commands, { selectionCount: 0 });
		expect(offered.map((entry) => entry.id)).not.toContain("file.rename");
	});

	test("a command whose clause holds is kept", () => {
		const offered = availableCommands(commands, { selectionCount: 1 });
		expect(offered.map((entry) => entry.id)).toContain("file.rename");
	});

	test("a command with no clause is always offered", () => {
		const offered = availableCommands(commands, {});
		expect(offered.map((entry) => entry.id)).toContain("file.newFolder");
	});
});

describe("ranking a palette query", () => {
	test("an empty query keeps every command", () => {
		expect(rankCommands(commands, "")).toHaveLength(commands.length);
	});

	test("the query's characters need only appear in order", () => {
		expect(titlesFor("ntrm")).toEqual(["Open terminal here"]);
	});

	test("a command the query cannot reach is dropped", () => {
		expect(titlesFor("zzz")).toEqual([]);
	});

	test("matching is case-insensitive in both directions", () => {
		expect(titlesFor("RENAME")).toEqual(["Rename"]);
	});

	test("a run of characters at the start outranks the same characters scattered", () => {
		expect(titlesFor("ne")).toEqual(["New folder", "Rename", "Open terminal here"]);
	});

	test("the category reaches a command its title does not spell", () => {
		expect(titlesFor("navigation open")).toEqual(["Open terminal here"]);
	});

	test("a title match outranks a match that needed the category", () => {
		expect(titlesFor("view")[0]).toBe("Grid view");
	});

	test("the highlights point at the characters that matched the title", () => {
		const [match] = rankCommands(commands, "grid");
		expect(match.highlights).toEqual([0, 1, 2, 3]);
	});

	test("a match found through the category highlights nothing", () => {
		const [match] = rankCommands(commands, "navigation open");
		expect(match.highlights).toEqual([]);
	});
});

describe("splitting a title for highlighting", () => {
	test("matched and unmatched characters become alternating runs", () => {
		expect(titleSegments("Rename", [0, 1])).toEqual([
			{ text: "Re", matched: true },
			{ text: "name", matched: false },
		]);
	});

	test("a title with nothing matched stays one run", () => {
		expect(titleSegments("Rename", [])).toEqual([{ text: "Rename", matched: false }]);
	});
});
