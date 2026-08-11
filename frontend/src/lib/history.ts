/**
 * Back/forward stack for visited directories, kept free of Svelte state so the
 * navigation rules stay testable. `index` points at the current location.
 */
export type History = {
	entries: string[];
	index: number;
};

export const emptyHistory: History = { entries: [], index: -1 };

/** Enough to cover a session's browsing without growing without bound. */
const historyLimit = 200;

/**
 * Records a visit. Revisiting the current location changes nothing, and moving
 * somewhere new after going back drops the forward entries.
 */
export function historyAfterVisit(current: History, path: string): History {
	if (currentLocation(current) === path) {
		return current;
	}
	const kept = current.entries.slice(0, current.index + 1);
	kept.push(path);
	return trimToLimit(kept);
}

export function canGoBack(history: History): boolean {
	return history.index > 0;
}

export function canGoForward(history: History): boolean {
	return history.index >= 0 && history.index < history.entries.length - 1;
}

export function historyAfterBack(history: History): History {
	if (!canGoBack(history)) {
		return history;
	}
	return { entries: history.entries, index: history.index - 1 };
}

export function historyAfterForward(history: History): History {
	if (!canGoForward(history)) {
		return history;
	}
	return { entries: history.entries, index: history.index + 1 };
}

export function currentLocation(history: History): string {
	if (history.index < 0) {
		return "";
	}
	return history.entries[history.index];
}

function trimToLimit(entries: string[]): History {
	const kept = entries.slice(Math.max(0, entries.length - historyLimit));
	return { entries: kept, index: kept.length - 1 };
}
