import { describe, expect, test } from "bun:test";
import { compileKeymap, resolveChord, shortcutFor } from "../frontend/src/lib/keymap";

const context = { listFocused: true, searchOpen: false, selectionCount: 1 };

describe("compileKeymap", () => {
	test("a binding keeps its command and clause", () => {
		expect(compileKeymap([{ key: "ctrl+c", command: "file.copy", when: "listFocused" }])).toEqual([
			{ chords: ["ctrl+c"], command: "file.copy", when: "listFocused" },
		]);
	});

	test("a missing clause compiles to an unconditional binding", () => {
		expect(compileKeymap([{ key: "f", command: "search.here" }])[0].when).toBe("");
	});

	// One bad line in a hand-edited settings file must not disable the rest.
	test("an unreadable key drops that binding only", () => {
		const keymap = compileKeymap([
			{ key: "hyper+q", command: "bad" },
			{ key: "f", command: "good" },
		]);
		expect(keymap).toHaveLength(1);
		expect(keymap[0].command).toBe("good");
	});
});

describe("resolveChord", () => {
	const keymap = compileKeymap([
		{ key: "f", command: "search.here" },
		{ key: "g g", command: "list.first" },
		{ key: "g h", command: "go.home" },
		{ key: "d d", command: "file.cut", when: "selectionCount > 0" },
	]);

	test("a bound chord runs its command", () => {
		expect(resolveChord(keymap, [], "f", context)).toEqual({
			status: "run",
			command: "search.here",
		});
	});

	test("an unbound chord resolves to nothing", () => {
		expect(resolveChord(keymap, [], "q", context)).toEqual({ status: "none" });
	});

	test("the start of a sequence waits for the rest", () => {
		expect(resolveChord(keymap, [], "g", context)).toEqual({ status: "pending" });
	});

	test("the second chord of a sequence runs it", () => {
		expect(resolveChord(keymap, ["g"], "g", context)).toEqual({
			status: "run",
			command: "list.first",
		});
		expect(resolveChord(keymap, ["g"], "h", context)).toEqual({
			status: "run",
			command: "go.home",
		});
	});

	test("a sequence that goes nowhere resolves to nothing", () => {
		expect(resolveChord(keymap, ["g"], "q", context)).toEqual({ status: "none" });
	});

	test("a clause that does not hold leaves the chord unbound", () => {
		expect(resolveChord(keymap, ["d"], "d", { ...context, selectionCount: 0 })).toEqual({
			status: "none",
		});
	});
});

describe("layering", () => {
	test("a later binding overrides an earlier one on the same key", () => {
		const keymap = compileKeymap([
			{ key: "f", command: "search.here" },
			{ key: "f", command: "custom.thing" },
		]);
		expect(resolveChord(keymap, [], "f", context)).toEqual({
			status: "run",
			command: "custom.thing",
		});
	});

	test("an empty command unbinds the key entirely", () => {
		const keymap = compileKeymap([
			{ key: "f", command: "search.here" },
			{ key: "f", command: "" },
		]);
		expect(resolveChord(keymap, [], "f", context)).toEqual({ status: "none" });
	});

	// Overriding only under a clause has to leave the original binding in place
	// everywhere else.
	test("an override with a clause applies only where the clause holds", () => {
		const keymap = compileKeymap([
			{ key: "f", command: "search.here" },
			{ key: "f", command: "custom.thing", when: "searchOpen" },
		]);
		expect(resolveChord(keymap, [], "f", context)).toEqual({
			status: "run",
			command: "search.here",
		});
		expect(resolveChord(keymap, [], "f", { ...context, searchOpen: true })).toEqual({
			status: "run",
			command: "custom.thing",
		});
	});

	// Waiting for the longer sequence would need a timeout, and a shortcut that
	// only fires after a pause is worse than one that cannot be shadowed.
	test("an exact match beats a longer sequence that starts the same way", () => {
		const keymap = compileKeymap([
			{ key: "d", command: "file.delete" },
			{ key: "d d", command: "file.cut" },
		]);
		expect(resolveChord(keymap, [], "d", context)).toEqual({
			status: "run",
			command: "file.delete",
		});
	});
});

describe("shortcutFor", () => {
	const keymap = compileKeymap([
		{ key: "ctrl+c", command: "file.copy" },
		{ key: "g g", command: "list.first" },
		{ key: "ctrl+shift+n", command: "file.newFolder" },
	]);

	test("a command's binding is rendered for a menu", () => {
		expect(shortcutFor(keymap, "file.copy", context)).toBe("Ctrl+C");
		expect(shortcutFor(keymap, "list.first", context)).toBe("G G");
		expect(shortcutFor(keymap, "file.newFolder", context)).toBe("Ctrl+Shift+N");
	});

	test("an unbound command has no label", () => {
		expect(shortcutFor(keymap, "file.openWith", context)).toBe("");
	});

	test("the winning binding is the one shown", () => {
		const overridden = compileKeymap([
			{ key: "ctrl+c", command: "file.copy" },
			{ key: "y y", command: "file.copy" },
		]);
		expect(shortcutFor(overridden, "file.copy", context)).toBe("Y Y");
	});
});
