/**
 * Finding an entry by the start of its name. The query comes from the find
 * prompt rather than from the listing's own key handling, so there is no idle
 * buffer here: what the user has typed is on screen.
 */

/**
 * Finds the entry to jump to, searching forwards from the current position and
 * wrapping around. A single character starts the search one past the current
 * entry, so repeating it steps through the matches instead of sticking to the
 * first one.
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
