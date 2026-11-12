/**
 * Name matching for the filter and the find prompt. Both are substring matches
 * with smart case: a query in lower case ignores case, and one with a capital
 * in it is exact.
 */
export function nameMatches(name: string, query: string): boolean {
	if (query === "") {
		return true;
	}
	if (query === query.toLowerCase()) {
		return name.toLowerCase().includes(query);
	}
	return name.includes(query);
}

export type FindDirection = "forward" | "backward";

/**
 * The next entry whose name matches, wrapping around. `includeCurrent` lets a
 * query that is still being typed stay on the entry it already matches. With no
 * cursor yet the scan starts from the first entry, or the last one going backwards.
 */
export function nextMatchIndex(
	names: string[],
	query: string,
	currentIndex: number,
	direction: FindDirection,
	includeCurrent: boolean,
): number {
	if (names.length === 0 || query === "") {
		return -1;
	}
	const step = stepOf(direction);
	const origin = originOf(currentIndex, direction, names.length);
	const firstOffset = firstOffsetOf(includeCurrent, currentIndex);
	for (let offset = firstOffset; offset < names.length + firstOffset; offset++) {
		const candidate = wrapped(origin + step * offset, names.length);
		if (nameMatches(names[candidate], query)) {
			return candidate;
		}
	}
	return -1;
}

function stepOf(direction: FindDirection): number {
	if (direction === "backward") {
		return -1;
	}
	return 1;
}

function originOf(currentIndex: number, direction: FindDirection, total: number): number {
	if (currentIndex >= 0) {
		return currentIndex;
	}
	if (direction === "backward") {
		return total;
	}
	return -1;
}

function firstOffsetOf(includeCurrent: boolean, currentIndex: number): number {
	if (includeCurrent && currentIndex >= 0) {
		return 0;
	}
	return 1;
}

function wrapped(index: number, total: number): number {
	return ((index % total) + total) % total;
}
