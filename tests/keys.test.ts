import { describe, expect, test } from "bun:test";
import {
	chordFromEvent,
	chordLabel,
	parseChord,
	parseKeySequence,
	sequenceLabel,
} from "../frontend/src/lib/keys";

function press(key: string, modifiers: Partial<Record<string, boolean>> = {}) {
	return {
		key,
		ctrlKey: modifiers.ctrl === true,
		altKey: modifiers.alt === true,
		shiftKey: modifiers.shift === true,
		metaKey: modifiers.meta === true,
	};
}

describe("chordFromEvent", () => {
	test("a plain letter is its own chord", () => {
		expect(chordFromEvent(press("f"))).toBe("f");
	});

	test("modifiers come out in a fixed order", () => {
		expect(chordFromEvent(press("c", { meta: true, ctrl: true, alt: true }))).toBe(
			"ctrl+alt+meta+c",
		);
	});

	test("a named key keeps its shift", () => {
		expect(chordFromEvent(press("Delete", { shift: true }))).toBe("shift+delete");
	});

	test("arrow keys lose the Arrow prefix", () => {
		expect(chordFromEvent(press("ArrowDown"))).toBe("down");
	});

	test("the space bar is a named key, not a blank one", () => {
		expect(chordFromEvent(press(" "))).toBe("space");
	});

	test("holding a modifier on its own is not a chord yet", () => {
		expect(chordFromEvent(press("Shift", { shift: true }))).toBe("");
	});

	test("an unknown named key produces nothing rather than a bad chord", () => {
		expect(chordFromEvent(press("Unidentified"))).toBe("");
	});
});

describe("shift folding", () => {
	// The character already says shift was held, and which physical key produced
	// it depends on the layout.
	test("a typed capital and a written shift+letter are the same chord", () => {
		expect(chordFromEvent(press("Z", { shift: true }))).toBe("Z");
		expect(parseChord("shift+z")).toBe("Z");
		expect(parseChord("Z")).toBe("Z");
	});

	test("case still separates two bindings", () => {
		expect(parseChord("z")).not.toBe(parseChord("Z"));
	});

	test("ctrl+shift+n matches the capital the browser reports", () => {
		expect(parseChord("ctrl+shift+n")).toBe(chordFromEvent(press("N", { ctrl: true, shift: true })));
	});

	test("a shifted punctuation character needs no shift modifier", () => {
		expect(chordFromEvent(press("?", { shift: true }))).toBe("?");
	});
});

describe("parseChord", () => {
	test("modifier aliases are accepted", () => {
		expect(parseChord("control+l")).toBe("ctrl+l");
		expect(parseChord("cmd+l")).toBe("meta+l");
	});

	test("case and spacing in the spec do not matter", () => {
		expect(parseChord("  CTRL + l ")).toBe("ctrl+l");
	});

	test("plus can itself be the key", () => {
		expect(parseChord("ctrl++")).toBe("ctrl++");
	});

	test("an unknown modifier is rejected", () => {
		expect(parseChord("hyper+l")).toBe("");
	});

	test("an empty spec is rejected", () => {
		expect(parseChord("   ")).toBe("");
	});
});

describe("parseKeySequence", () => {
	test("spaces separate the chords of a sequence", () => {
		expect(parseKeySequence("g g")).toEqual(["g", "g"]);
	});

	test("a single chord is a sequence of one", () => {
		expect(parseKeySequence("ctrl+c")).toEqual(["ctrl+c"]);
	});

	// Half a sequence would silently bind the readable half to the command.
	test("one unreadable chord rejects the whole sequence", () => {
		expect(parseKeySequence("g hyper+g")).toEqual([]);
	});
});

describe("labels", () => {
	test("a capital letter is shown as a shifted press", () => {
		expect(chordLabel("Z")).toBe("Shift+Z");
	});

	test("a lowercase letter is shown without shift", () => {
		expect(chordLabel("ctrl+c")).toBe("Ctrl+C");
	});

	test("named keys use their display name", () => {
		expect(chordLabel("shift+delete")).toBe("Shift+Delete");
		expect(chordLabel("down")).toBe("↓");
		expect(chordLabel("f2")).toBe("F2");
	});

	test("a sequence is labelled chord by chord", () => {
		expect(sequenceLabel(["g", "g"])).toBe("G G");
	});
});
