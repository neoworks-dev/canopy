import { describe, expect, test } from "bun:test";
import {
	allMarked,
	cursorMoved,
	markToggled,
	marksInverted,
	visualMoved,
	visualStarted,
	type MarkState,
} from "../ui/lib/marks";
import { selectionAfterJump, emptySelection } from "../ui/lib/selection";

const paths = ["/a", "/b", "/c", "/d", "/e"];

function cursorAt(index: number): MarkState {
	return { selection: selectionAfterJump(emptySelection, paths, index, false), isMarking: false };
}

describe("cursorMoved", () => {
	test("without marks the entry under the cursor is the selection", () => {
		const moved = cursorMoved(cursorAt(1), paths, 3);
		expect(moved.selection.paths).toEqual(["/d"]);
		expect(moved.isMarking).toBe(false);
	});

	test("marks stay put while the cursor moves", () => {
		const marked = markToggled(cursorAt(1), paths);
		const moved = cursorMoved(marked, paths, 3);
		expect(moved.selection.paths).toEqual(["/b"]);
		expect(moved.selection.focusIndex).toBe(3);
		expect(moved.isMarking).toBe(true);
	});

	test("a target outside the list leaves the state alone", () => {
		const state = cursorAt(1);
		expect(cursorMoved(state, paths, 9)).toBe(state);
	});
});

describe("markToggled", () => {
	test("marks the entry under the cursor", () => {
		const marked = markToggled(cursorAt(2), paths);
		expect(marked.selection.paths).toEqual(["/c"]);
		expect(marked.isMarking).toBe(true);
	});

	test("adds to existing marks", () => {
		let state = markToggled(cursorAt(0), paths);
		state = cursorMoved(state, paths, 2);
		state = markToggled(state, paths);
		expect(state.selection.paths).toEqual(["/a", "/c"]);
	});

	test("unmarking the last mark returns to the cursor's entry", () => {
		const marked = markToggled(cursorAt(2), paths);
		const unmarked = markToggled(marked, paths);
		expect(unmarked.isMarking).toBe(false);
		expect(unmarked.selection.paths).toEqual(["/c"]);
	});
});

describe("marksInverted and allMarked", () => {
	test("inverting flips which entries are marked", () => {
		let state = markToggled(cursorAt(0), paths);
		state = marksInverted(state, paths);
		expect(state.selection.paths).toEqual(["/b", "/c", "/d", "/e"]);
		expect(state.isMarking).toBe(true);
	});

	test("with nothing marked, inverting marks everything", () => {
		expect(marksInverted(cursorAt(1), paths).selection.paths).toEqual(paths);
	});

	test("marks every entry", () => {
		expect(allMarked(cursorAt(1), paths).selection.paths).toEqual(paths);
	});
});

describe("visual mode", () => {
	test("sweeps a range from where it started to the cursor, both ends included", () => {
		const visual = visualStarted(cursorAt(1), "select");
		expect(visualMoved(visual, paths, 3).selection.paths).toEqual(["/b", "/c", "/d"]);
	});

	test("the sweep follows the cursor back", () => {
		const visual = visualStarted(cursorAt(1), "select");
		expect(visualMoved(visual, paths, 3).selection.paths).toHaveLength(3);
		expect(visualMoved(visual, paths, 1).selection.paths).toEqual(["/b"]);
	});

	test("keeps the marks held before the sweep began", () => {
		let state = markToggled(cursorAt(0), paths);
		state = cursorMoved(state, paths, 3);
		const visual = visualStarted(state, "select");
		expect(visualMoved(visual, paths, 4).selection.paths).toEqual(["/a", "/d", "/e"]);
	});

	test("an unsetting sweep removes entries from the marks", () => {
		const everythingMarked = allMarked(cursorAt(1), paths);
		const state = cursorMoved(everythingMarked, paths, 1);
		const visual = visualStarted(state, "unset");
		expect(visualMoved(visual, paths, 2).selection.paths).toEqual(["/a", "/d", "/e"]);
	});
});
