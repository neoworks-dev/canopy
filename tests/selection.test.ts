import { describe, expect, test } from "bun:test";
import {
	emptySelection,
	pruneSelection,
	selectAll,
	selectionAfterClick,
	selectionAfterJump,
	selectionAfterMove,
	selectionAfterRelisting,
	type Selection,
} from "../frontend/src/lib/selection";

const paths = ["/a", "/b", "/c", "/d", "/e"];

const plainClick = { toggle: false, range: false };
const controlClick = { toggle: true, range: false };
const shiftClick = { toggle: false, range: true };
const controlShiftClick = { toggle: true, range: true };

function selectionOf(pathList: string[], anchorIndex: number, focusIndex: number): Selection {
	return { paths: pathList, anchorIndex, focusIndex };
}

describe("selectionAfterClick", () => {
	test("a plain click replaces the selection", () => {
		const selection = selectionAfterClick(selectionOf(["/a", "/b"], 0, 1), paths, 3, plainClick);
		expect(selection.paths).toEqual(["/d"]);
		expect(selection.anchorIndex).toBe(3);
	});

	test("ctrl-click adds an entry without dropping the others", () => {
		const selection = selectionAfterClick(selectionOf(["/a"], 0, 0), paths, 2, controlClick);
		expect(selection.paths).toEqual(["/a", "/c"]);
	});

	test("ctrl-click on a selected entry deselects only that entry", () => {
		const selection = selectionAfterClick(selectionOf(["/a", "/c"], 0, 2), paths, 0, controlClick);
		expect(selection.paths).toEqual(["/c"]);
	});

	test("shift-click selects the range from the anchor", () => {
		const selection = selectionAfterClick(selectionOf(["/b"], 1, 1), paths, 3, shiftClick);
		expect(selection.paths).toEqual(["/b", "/c", "/d"]);
		expect(selection.anchorIndex).toBe(1);
	});

	test("shift-click backwards keeps the anchor at the far end", () => {
		const selection = selectionAfterClick(selectionOf(["/d"], 3, 3), paths, 1, shiftClick);
		expect(selection.paths).toEqual(["/b", "/c", "/d"]);
		expect(selection.anchorIndex).toBe(3);
	});

	test("a second shift-click reshapes the range instead of growing it", () => {
		const firstRange = selectionAfterClick(selectionOf(["/a"], 0, 0), paths, 3, shiftClick);
		const secondRange = selectionAfterClick(firstRange, paths, 1, shiftClick);
		expect(secondRange.paths).toEqual(["/a", "/b"]);
	});

	test("ctrl-shift-click merges the range into the existing selection", () => {
		const selection = selectionAfterClick(selectionOf(["/e"], 4, 4), paths, 1, controlShiftClick);
		expect(selection.paths).toEqual(["/e", "/b", "/c", "/d"]);
	});

	test("shift-click without an anchor selects just the clicked entry", () => {
		const selection = selectionAfterClick(emptySelection, paths, 2, shiftClick);
		expect(selection.paths).toEqual(["/c"]);
	});

	test("an out-of-range index leaves the selection untouched", () => {
		const current = selectionOf(["/a"], 0, 0);
		expect(selectionAfterClick(current, paths, 9, plainClick)).toBe(current);
	});
});

describe("selectionAfterMove", () => {
	test("moves the selection down by one", () => {
		const selection = selectionAfterMove(selectionOf(["/b"], 1, 1), paths, 1, false);
		expect(selection.paths).toEqual(["/c"]);
		expect(selection.focusIndex).toBe(2);
	});

	test("shift extends from the anchor while the focus moves", () => {
		const first = selectionAfterMove(selectionOf(["/b"], 1, 1), paths, 1, true);
		const second = selectionAfterMove(first, paths, 1, true);
		expect(second.paths).toEqual(["/b", "/c", "/d"]);
		expect(second.anchorIndex).toBe(1);
	});

	test("stops at the ends of the list", () => {
		const atTop = selectionOf(["/a"], 0, 0);
		expect(selectionAfterMove(atTop, paths, -1, false)).toBe(atTop);
	});

	test("starts at the first entry when nothing is selected", () => {
		const selection = selectionAfterMove(emptySelection, paths, 1, false);
		expect(selection.paths).toEqual(["/a"]);
	});
});

describe("selectionAfterJump", () => {
	test("jumps straight to an entry", () => {
		const selection = selectionAfterJump(selectionOf(["/a"], 0, 0), paths, 4, false);
		expect(selection.paths).toEqual(["/e"]);
		expect(selection.focusIndex).toBe(4);
	});

	test("extending a jump covers everything from the anchor", () => {
		const selection = selectionAfterJump(selectionOf(["/b"], 1, 1), paths, 4, true);
		expect(selection.paths).toEqual(["/b", "/c", "/d", "/e"]);
		expect(selection.anchorIndex).toBe(1);
	});

	test("an out-of-range target changes nothing", () => {
		const current = selectionOf(["/a"], 0, 0);
		expect(selectionAfterJump(current, paths, 9, false)).toBe(current);
	});
});

describe("selectAll", () => {
	test("selects every listed entry", () => {
		expect(selectAll(paths).paths).toEqual(paths);
	});

	test("an empty list yields an empty selection", () => {
		expect(selectAll([])).toEqual(emptySelection);
	});
});

describe("pruneSelection", () => {
	test("drops entries that are no longer listed", () => {
		const selection = pruneSelection(selectionOf(["/a", "/gone"], 0, 1), paths);
		expect(selection.paths).toEqual(["/a"]);
		expect(selection.focusIndex).toBe(0);
	});

	test("keeps the same object when everything still exists", () => {
		const current = selectionOf(["/a", "/b"], 0, 1);
		expect(pruneSelection(current, paths)).toBe(current);
	});

	test("empties the selection when nothing survives", () => {
		expect(pruneSelection(selectionOf(["/gone"], 0, 0), paths)).toEqual(emptySelection);
	});
});

describe("selectionAfterRelisting", () => {
	test("the focus follows its entry when one is inserted before it", () => {
		const withInsert = ["/a", "/a2", "/b", "/c", "/d", "/e"];
		const selection = selectionAfterRelisting(selectionOf(["/c"], 2, 2), paths, withInsert);
		expect(selection.focusIndex).toBe(3);
		expect(selection.anchorIndex).toBe(3);
	});

	test("the focus follows its entry when one before it disappears", () => {
		const withRemoval = ["/b", "/c", "/d", "/e"];
		const selection = selectionAfterRelisting(selectionOf(["/c"], 2, 2), paths, withRemoval);
		expect(selection.focusIndex).toBe(1);
	});

	test("drops entries that are gone and keeps the rest", () => {
		const withRemoval = ["/a", "/c", "/d", "/e"];
		const selection = selectionAfterRelisting(selectionOf(["/a", "/b"], 0, 1), paths, withRemoval);
		expect(selection.paths).toEqual(["/a"]);
		expect(selection.focusIndex).toBe(-1);
	});

	test("empties the selection when nothing survives", () => {
		const selection = selectionAfterRelisting(selectionOf(["/b"], 1, 1), paths, ["/a"]);
		expect(selection).toEqual(emptySelection);
	});

	test("an empty selection stays empty", () => {
		expect(selectionAfterRelisting(emptySelection, paths, paths)).toEqual(emptySelection);
	});
});
