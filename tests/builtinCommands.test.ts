import { describe, expect, test } from "bun:test";
import {
	backgroundMenuIds,
	defaultKeymap,
	movementCommands,
	selectionMenuIds,
	sortCommands,
	staticCommandDescriptors,
} from "../ui/lib/builtinCommands";
import { compileKeymap } from "../ui/lib/keymap";
import { whenClauseIsValid } from "../ui/lib/whenClause";

const builtinIds = new Set([
	...staticCommandDescriptors.map((descriptor) => descriptor.id),
	...movementCommands.map((command) => command.id),
]);

describe("the built-in command set", () => {
	test("no command id is defined twice", () => {
		const ids = [
			...staticCommandDescriptors.map((descriptor) => descriptor.id),
			...movementCommands.map((command) => command.id),
		];
		expect(ids).toHaveLength(builtinIds.size);
	});

	test("every clause parses, so no command silently disables itself", () => {
		const broken = staticCommandDescriptors.filter(
			(descriptor) => !whenClauseIsValid(descriptor.when ?? ""),
		);
		expect(broken.map((descriptor) => descriptor.id)).toEqual([]);
	});

	test("both a plain and an extending command exist for every movement", () => {
		expect(movementCommands.filter((command) => command.extend)).toHaveLength(10);
		expect(movementCommands.filter((command) => !command.extend)).toHaveLength(10);
	});
});

describe("the default keymap", () => {
	test("every key compiles, so no default binding is silently dropped", () => {
		expect(compileKeymap(defaultKeymap)).toHaveLength(defaultKeymap.length);
	});

	test("every binding points at a command that exists", () => {
		const dangling = defaultKeymap.filter((binding) => !builtinIds.has(binding.command));
		expect(dangling.map((binding) => `${binding.key} -> ${binding.command}`)).toEqual([]);
	});

	test("every clause parses, so no default binding is dead", () => {
		const broken = defaultKeymap.filter((binding) => !whenClauseIsValid(binding.when ?? ""));
		expect(broken.map((binding) => binding.key)).toEqual([]);
	});

	// Two bindings on one key under the same conditions means the earlier one is
	// unreachable, which is a mistake rather than a deliberate override.
	test("no two bindings shadow each other outright", () => {
		const seen = new Set<string>();
		const shadowed: string[] = [];
		for (const binding of compileKeymap(defaultKeymap)) {
			const signature = `${binding.chords.join(" ")}|${binding.when}`;
			appendIfSeen(seen, shadowed, signature);
		}
		expect(shadowed).toEqual([]);
	});

	test("the letters the modal layout promises are bound", () => {
		expect(commandFor("f")).toBe("list.filter");
		expect(commandFor("s")).toBe("search.here");
		expect(commandFor("/")).toBe("list.find");
		expect(commandFor("n")).toBe("list.findNext");
		expect(commandFor("Z")).toBe("zoxide.jump");
		expect(commandFor("h")).toBe("navigate.parent");
		expect(commandFor("l")).toBe("navigate.enter");
	});

	test("l and right only enter folders, while enter and o open files too", () => {
		expect(commandFor("l")).toBe("navigate.enter");
		expect(commandFor("right")).toBe("navigate.enter");
		expect(commandFor("enter")).toBe("navigate.open");
		expect(commandFor("o")).toBe("navigate.open");
	});

	test("Yazi's copy, seek, tab, help and bulk keys are bound", () => {
		expect(commandFor("c c")).toBe("copy.path");
		expect(commandFor("c C")).toBe("copy.url");
		expect(commandFor("c d")).toBe("copy.dirpath");
		expect(commandFor("c D")).toBe("copy.dirurl");
		expect(commandFor("c f")).toBe("copy.filename");
		expect(commandFor("c n")).toBe("copy.nameWithoutExtension");
		expect(commandFor("K")).toBe("preview.seekUp");
		expect(commandFor("J")).toBe("preview.seekDown");
		expect(commandFor("t r")).toBe("tab.rename");
		expect(commandFor("~")).toBe("help.open");
		expect(commandFor("f1")).toBe("help.open");
		expect(commandFor("A")).toBe("file.bulkCreate");
		expect(commandFor("shift+enter")).toBe("navigate.openWith");
		expect(commandFor("-")).toBe("file.symlink");
		expect(commandFor("_")).toBe("file.symlinkRelative");
		expect(commandFor("ctrl+-")).toBe("file.hardlink");
		expect(commandFor(";")).toBe("shell.run");
		expect(commandFor(":")).toBe("shell.runBlocking");
		expect(commandFor("g f")).toBe("navigate.followLink");
		expect(commandFor("S")).toBe("search.contents");
		expect(commandFor(", e")).toBe("view.sort.extension.ascending");
		expect(commandFor(", N")).toBe("view.sort.natural.descending");
		expect(commandFor(", b")).toBe("view.sort.created.ascending");
		expect(commandFor(", r")).toBe("view.sort.random");
		expect(commandFor("m b")).toBe("view.linemode.created");
		expect(commandFor("m o")).toBe("view.linemode.owner");
		expect(commandFor("tab")).toBe("list.spot");
		expect(commandFor("w")).toBe("tasks.show");
	});

	test("sorting by time or size also shows that column, as Yazi's linemode does", () => {
		const bySize = sortCommands.find((command) => command.id === "view.sort.size.descending");
		const byName = sortCommands.find((command) => command.id === "view.sort.name.ascending");
		expect(bySize?.linemodeId).toBe("view.linemode.size");
		expect(byName?.linemodeId).toBe("");
	});

	test("copy and undo answer both the Yazi letter and the desktop chord", () => {
		expect(commandFor("y")).toBe("file.copy");
		expect(commandFor("ctrl+c")).toBe("file.copy");
		expect(commandFor("u")).toBe("file.undo");
		expect(commandFor("ctrl+z")).toBe("file.undo");
	});

	test("the clipboard keys are Yazi's single letters", () => {
		expect(commandFor("y")).toBe("file.copy");
		expect(commandFor("x")).toBe("file.cut");
		expect(commandFor("p")).toBe("file.paste");
		expect(commandFor("d")).toBe("file.trash");
		expect(commandFor("D")).toBe("file.deletePermanently");
	});

	test("sequences are reachable: no key is both a command and the start of a sequence", () => {
		const compiled = compileKeymap(defaultKeymap);
		const unreachable: string[] = [];
		for (const binding of compiled) {
			collectShadowedSequences(compiled, binding, unreachable);
		}
		expect(unreachable).toEqual([]);
	});

	test("every digit selects its tab", () => {
		for (let number = 1; number <= 9; number++) {
			expect(commandFor(String(number))).toBe(`tab.select.${number}`);
		}
	});

	test("each sort key has an ascending lower case and a descending capital", () => {
		expect(commandFor(", a")).toBe("view.sort.name.ascending");
		expect(commandFor(", A")).toBe("view.sort.name.descending");
		expect(commandFor(", s")).toBe("view.sort.size.ascending");
		expect(commandFor(", M")).toBe("view.sort.modified.descending");
	});
});

describe("the context menus", () => {
	const menuIds = [...selectionMenuIds.flat(), ...backgroundMenuIds.flat()];

	test("every menu entry names a command that exists", () => {
		expect(menuIds.filter((id) => !builtinIds.has(id))).toEqual([]);
	});
});

function collectShadowedSequences(
	compiled: ReturnType<typeof compileKeymap>,
	binding: ReturnType<typeof compileKeymap>[number],
	unreachable: string[],
) {
	if (binding.chords.length < 2) {
		return;
	}
	const prefix = binding.chords.slice(0, -1);
	const shadow = compiled.find(
		(candidate) =>
			candidate.chords.length === prefix.length &&
			candidate.chords.every((chord, index) => chord === prefix[index]) &&
			candidate.when === binding.when,
	);
	if (shadow) {
		unreachable.push(`${binding.chords.join(" ")} is shadowed by ${prefix.join(" ")}`);
	}
}

function commandFor(key: string): string {
	const bound = defaultKeymap.filter((binding) => binding.key === key);
	return bound[bound.length - 1].command;
}

function appendIfSeen(seen: Set<string>, shadowed: string[], signature: string) {
	if (seen.has(signature)) {
		shadowed.push(signature);
		return;
	}
	seen.add(signature);
}
