/**
 * Selection rules for the entry list, kept free of Svelte state so the click and
 * keyboard behaviour stays testable. Paths identify entries; the indices track
 * where a shift-range starts (anchor) and where it currently ends (focus).
 */
export type Selection = {
	paths: string[];
	anchorIndex: number;
	focusIndex: number;
};

export type SelectionModifiers = {
	/** Ctrl or Cmd: add or remove one entry, keeping the rest. */
	toggle: boolean;
	/** Shift: extend from the anchor to the clicked entry. */
	range: boolean;
};

export const emptySelection: Selection = { paths: [], anchorIndex: -1, focusIndex: -1 };

export function selectionAfterClick(
	current: Selection,
	allPaths: string[],
	index: number,
	modifiers: SelectionModifiers,
): Selection {
	if (!isWithinBounds(allPaths, index)) {
		return current;
	}
	if (modifiers.range) {
		return extendSelection(current, allPaths, index, modifiers.toggle);
	}
	if (modifiers.toggle) {
		return toggleSelection(current, allPaths, index);
	}
	return selectOnly(allPaths, index);
}

/** Arrow-key movement. `step` is -1 or 1; shift extends instead of replacing. */
export function selectionAfterMove(
	current: Selection,
	allPaths: string[],
	step: number,
	extend: boolean,
): Selection {
	const nextIndex = current.focusIndex + step;
	if (!isWithinBounds(allPaths, nextIndex)) {
		return current;
	}
	if (extend) {
		return extendSelection(current, allPaths, nextIndex, false);
	}
	return selectOnly(allPaths, nextIndex);
}

export function selectAll(allPaths: string[]): Selection {
	if (allPaths.length === 0) {
		return emptySelection;
	}
	return { paths: [...allPaths], anchorIndex: 0, focusIndex: allPaths.length - 1 };
}

/** Drops entries that are no longer listed, e.g. after hiding dotfiles. */
export function pruneSelection(current: Selection, allPaths: string[]): Selection {
	const remaining = current.paths.filter((path) => allPaths.includes(path));
	if (remaining.length === current.paths.length) {
		return current;
	}
	if (remaining.length === 0) {
		return emptySelection;
	}
	const lastIndex = allPaths.indexOf(remaining[remaining.length - 1]);
	return { paths: remaining, anchorIndex: lastIndex, focusIndex: lastIndex };
}

function selectOnly(allPaths: string[], index: number): Selection {
	return { paths: [allPaths[index]], anchorIndex: index, focusIndex: index };
}

function toggleSelection(current: Selection, allPaths: string[], index: number): Selection {
	const path = allPaths[index];
	if (current.paths.includes(path)) {
		const remaining = current.paths.filter((selectedPath) => selectedPath !== path);
		return { paths: remaining, anchorIndex: index, focusIndex: index };
	}
	return { paths: [...current.paths, path], anchorIndex: index, focusIndex: index };
}

function extendSelection(
	current: Selection,
	allPaths: string[],
	index: number,
	keepExisting: boolean,
): Selection {
	const anchorIndex = anchorFor(current, index);
	const range = rangeBetween(allPaths, anchorIndex, index);
	if (!keepExisting) {
		return { paths: range, anchorIndex, focusIndex: index };
	}
	return { paths: mergeUnique(current.paths, range), anchorIndex, focusIndex: index };
}

function anchorFor(current: Selection, fallbackIndex: number): number {
	if (current.anchorIndex < 0) {
		return fallbackIndex;
	}
	return current.anchorIndex;
}

function rangeBetween(allPaths: string[], firstIndex: number, secondIndex: number): string[] {
	const start = Math.min(firstIndex, secondIndex);
	const end = Math.max(firstIndex, secondIndex);
	return allPaths.slice(start, end + 1);
}

function mergeUnique(existing: string[], added: string[]): string[] {
	const merged = [...existing];
	for (const path of added) {
		if (!merged.includes(path)) {
			merged.push(path);
		}
	}
	return merged;
}

function isWithinBounds(allPaths: string[], index: number): boolean {
	return index >= 0 && index < allPaths.length;
}
