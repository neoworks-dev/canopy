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
	return selectionAfterJump(current, allPaths, current.focusIndex + step, extend);
}

/**
 * Moves the selection to one specific entry, which is what grid arrows,
 * Home/End, PageUp/PageDown and type-ahead all resolve to.
 */
export function selectionAfterJump(
	current: Selection,
	allPaths: string[],
	index: number,
	extend: boolean,
): Selection {
	if (!isWithinBounds(allPaths, index)) {
		return current;
	}
	if (extend) {
		return extendSelection(current, allPaths, index, false);
	}
	return selectOnly(allPaths, index);
}

/**
 * Selection from a rubber band. `keep` holds the paths that were already
 * selected when the drag started, which is how a ctrl-drag adds to them.
 */
export function selectionFromIndexes(
	allPaths: string[],
	indexes: number[],
	keep: string[],
): Selection {
	const covered = indexes.filter((index) => isWithinBounds(allPaths, index));
	const paths = mergeUnique(keep, covered.map((index) => allPaths[index]));
	if (covered.length === 0) {
		return { paths, anchorIndex: -1, focusIndex: -1 };
	}
	return {
		paths,
		anchorIndex: covered[0],
		focusIndex: covered[covered.length - 1],
	};
}

/**
 * Where the cursor lands in a folder just read, as Yazi places it: on the
 * folder just stepped out of, else on the first entry; -1 when it is empty.
 */
export function cursorIndexAfterLoad(paths: string[], cameFrom: string, folder: string): number {
	if (paths.length === 0) {
		return -1;
	}
	const childIndex = paths.indexOf(cameFrom);
	if (childIndex >= 0 && parentOf(cameFrom) === folder) {
		return childIndex;
	}
	return 0;
}

function parentOf(path: string): string {
	const lastSeparator = path.lastIndexOf("/");
	if (lastSeparator <= 0) {
		return "/";
	}
	return path.slice(0, lastSeparator);
}

/** Whether two selections hold the same paths in the same order, with the same anchor and focus. */
export function isSameSelection(first: Selection, second: Selection): boolean {
	if (first.anchorIndex !== second.anchorIndex || first.focusIndex !== second.focusIndex) {
		return false;
	}
	if (first.paths.length !== second.paths.length) {
		return false;
	}
	return first.paths.every((path, index) => path === second.paths[index]);
}

export function selectAll(allPaths: string[]): Selection {
	if (allPaths.length === 0) {
		return emptySelection;
	}
	return { paths: [...allPaths], anchorIndex: 0, focusIndex: allPaths.length - 1 };
}

/**
 * Follows the entries the anchor and focus pointed at after the listing was
 * re-read, so an entry created or deleted by something else shifts the indexes
 * without moving the focus ring onto a different entry.
 */
export function selectionAfterRelisting(
	current: Selection,
	previousPaths: string[],
	allPaths: string[],
): Selection {
	const paths = current.paths.filter((path) => allPaths.includes(path));
	if (paths.length === 0) {
		return emptySelection;
	}
	return {
		paths,
		anchorIndex: followIndex(current.anchorIndex, previousPaths, allPaths),
		focusIndex: followIndex(current.focusIndex, previousPaths, allPaths),
	};
}

function followIndex(index: number, previousPaths: string[], allPaths: string[]): number {
	if (!isWithinBounds(previousPaths, index)) {
		return -1;
	}
	return allPaths.indexOf(previousPaths[index]);
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
