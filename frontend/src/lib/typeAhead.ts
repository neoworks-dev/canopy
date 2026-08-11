/**
 * Type-ahead: typing printable characters over the listing jumps to the first
 * entry with that prefix. The buffer resets after a short idle pause, so a new
 * word starts a new search rather than extending the previous one.
 */
export type TypeAhead = {
	query: string;
	updatedAt: number;
};

export const emptyTypeAhead: TypeAhead = { query: "", updatedAt: 0 };

export const typeAheadResetMilliseconds = 900;

/** Keys that carry a modifier belong to a shortcut, not to a name. */
export type TypeAheadKey = {
	key: string;
	ctrlKey: boolean;
	metaKey: boolean;
	altKey: boolean;
};

export function isTypeAheadKey(event: TypeAheadKey): boolean {
	if (event.ctrlKey || event.metaKey || event.altKey) {
		return false;
	}
	return [...event.key].length === 1 && event.key !== " ";
}

export function typeAheadAfterKey(current: TypeAhead, character: string, now: number): TypeAhead {
	if (now - current.updatedAt > typeAheadResetMilliseconds) {
		return { query: character.toLowerCase(), updatedAt: now };
	}
	return { query: current.query + character.toLowerCase(), updatedAt: now };
}

/**
 * Finds the entry to jump to, searching forwards from the current position and
 * wrapping around. A single repeated character cycles through the matches
 * instead of sticking to the first one.
 */
export function typeAheadMatch(names: string[], query: string, currentIndex: number): number {
	if (names.length === 0 || query === "") {
		return -1;
	}
	const startIndex = searchStart(query, currentIndex, names.length);
	for (let offset = 0; offset < names.length; offset++) {
		const candidate = (startIndex + offset) % names.length;
		if (names[candidate].toLowerCase().startsWith(query)) {
			return candidate;
		}
	}
	return -1;
}

function searchStart(query: string, currentIndex: number, total: number): number {
	if (currentIndex < 0) {
		return 0;
	}
	if (query.length === 1) {
		return (currentIndex + 1) % total;
	}
	return currentIndex;
}
