/**
 * The cursor and the marks are separate: moving the cursor never changes which
 * entries are marked, and an action applies to the marks, or to the entry under
 * the cursor while nothing is marked. `Selection.paths` always holds the entries
 * an action would apply to, so everything that reads it keeps working; `isMarking`
 * says whether those are marks or just the cursor's entry.
 */
import { selectionAfterJump, type Selection } from "./selection";

export type MarkState = {
	selection: Selection;
	isMarking: boolean;
};

export type VisualMode = "select" | "unset";

/** A range being swept from `anchorIndex` to the cursor, on top of the marks held at the start. */
export type Visual = {
	mode: VisualMode;
	base: string[];
	anchorIndex: number;
};

/** Moves the cursor; marks stay where they are. */
export function cursorMoved(state: MarkState, allPaths: string[], index: number): MarkState {
	if (index < 0 || index >= allPaths.length) {
		return state;
	}
	if (!state.isMarking) {
		return { selection: selectionAfterJump(state.selection, allPaths, index, false), isMarking: false };
	}
	return { selection: { ...state.selection, anchorIndex: index, focusIndex: index }, isMarking: true };
}

/** Marks or unmarks the entry under the cursor. */
export function markToggled(state: MarkState, allPaths: string[]): MarkState {
	const focusIndex = state.selection.focusIndex;
	if (focusIndex < 0 || focusIndex >= allPaths.length) {
		return state;
	}
	const path = allPaths[focusIndex];
	const marked = markedPaths(state);
	if (marked.includes(path)) {
		return marksReplaced(allPaths, marked.filter((candidate) => candidate !== path), focusIndex);
	}
	return marksReplaced(allPaths, [...marked, path], focusIndex);
}

export function marksInverted(state: MarkState, allPaths: string[]): MarkState {
	const marked = markedPaths(state);
	const inverted = allPaths.filter((path) => !marked.includes(path));
	return marksReplaced(allPaths, inverted, state.selection.focusIndex);
}

export function allMarked(state: MarkState, allPaths: string[]): MarkState {
	return marksReplaced(allPaths, allPaths, state.selection.focusIndex);
}

/** Starts a sweep at the cursor, which is already inside it. */
export function visualStarted(state: MarkState, mode: VisualMode): Visual {
	return { mode, base: markedPaths(state), anchorIndex: state.selection.focusIndex };
}

/** The marks for a sweep that currently ends at `index`. */
export function visualMoved(
	visual: Visual,
	allPaths: string[],
	index: number,
): MarkState {
	if (index < 0 || index >= allPaths.length || visual.anchorIndex < 0) {
		return marksReplaced(allPaths, visual.base, visual.anchorIndex);
	}
	const start = Math.min(visual.anchorIndex, index);
	const end = Math.max(visual.anchorIndex, index);
	const swept = allPaths.slice(start, end + 1);
	return marksReplaced(allPaths, sweptMarks(visual, swept), index);
}

function sweptMarks(visual: Visual, swept: string[]): string[] {
	if (visual.mode === "unset") {
		return visual.base.filter((path) => !swept.includes(path));
	}
	const added = swept.filter((path) => !visual.base.includes(path));
	return [...visual.base, ...added];
}

function markedPaths(state: MarkState): string[] {
	if (!state.isMarking) {
		return [];
	}
	return state.selection.paths;
}

// Without any marks left the cursor's own entry is the target again.
function marksReplaced(allPaths: string[], paths: string[], focusIndex: number): MarkState {
	if (paths.length === 0) {
		return {
			selection: {
				paths: pathAt(allPaths, focusIndex),
				anchorIndex: focusIndex,
				focusIndex,
			},
			isMarking: false,
		};
	}
	return { selection: { paths, anchorIndex: focusIndex, focusIndex }, isMarking: true };
}

function pathAt(allPaths: string[], index: number): string[] {
	if (index < 0 || index >= allPaths.length) {
		return [];
	}
	return [allPaths[index]];
}
