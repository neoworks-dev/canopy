/**
 * How the listing is ordered. The backend does the sorting, so this mirrors the
 * order it takes and holds the rules for what a header click means.
 */
export type SortKey =
	| "name"
	| "natural"
	| "extension"
	| "size"
	| "modified"
	| "created"
	| "permissions"
	| "random";

export type SortOrder = {
	key: SortKey;
	descending: boolean;
	/** Independent of the key, so sorting by size still groups folders. */
	directoriesFirst: boolean;
};

export const defaultSortOrder: SortOrder = {
	key: "name",
	descending: false,
	directoriesFirst: true,
};

/**
 * Clicking the active column reverses it; clicking another one sorts by that
 * column ascending, which is where every file manager starts.
 */
export function sortOrderAfterHeaderClick(current: SortOrder, key: SortKey): SortOrder {
	if (current.key !== key) {
		return { ...current, key, descending: false };
	}
	return { ...current, descending: !current.descending };
}

/** Sorts by one key in a given direction, whatever the order was before. */
export function sortOrderBy(current: SortOrder, key: SortKey, descending: boolean): SortOrder {
	return { ...current, key, descending };
}

/** Flips the direction without changing which column the listing is sorted by. */
export function sortOrderReversed(current: SortOrder): SortOrder {
	return { ...current, descending: !current.descending };
}

export function sortOrderWithDirectoriesFirst(
	current: SortOrder,
	directoriesFirst: boolean,
): SortOrder {
	return { ...current, directoriesFirst };
}

/** Which way the arrow on a header points, or none when it is not the active one. */
export function sortDirectionOf(current: SortOrder, key: SortKey): "ascending" | "descending" | "" {
	if (current.key !== key) {
		return "";
	}
	if (current.descending) {
		return "descending";
	}
	return "ascending";
}
