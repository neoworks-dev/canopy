/**
 * Naming and describing the queries kept in the sidebar. A saved search holds
 * the query and the folder it was confined to, so opening one runs it again
 * rather than showing what it once found.
 */
import type { searches } from "../backend/models";

export type SavedSearch = searches.Search;

/**
 * The name a search is offered under when the user has not given one. The query
 * is what they typed, so it is what they will recognise; a scoped search says
 * where it looked, since the same query means different things in different
 * folders.
 */
export function suggestedName(query: string, scope: string): string {
	const trimmed = query.trim();
	if (trimmed === "" || scope === "") {
		return trimmed;
	}
	return `${trimmed} in ${lastSegmentOf(scope)}`;
}

function lastSegmentOf(path: string): string {
	const segments = path.split("/").filter((segment) => segment !== "");
	if (segments.length === 0) {
		return "/";
	}
	return segments[segments.length - 1];
}

/** What the sidebar shows on hovering a saved search. */
export function searchDescription(search: SavedSearch): string {
	if (search.scope === "") {
		return `${search.query} · everywhere`;
	}
	return `${search.query} · in ${search.scope}`;
}

/** Whether a name is free, so saving cannot silently replace another search. */
export function isNameTaken(saved: SavedSearch[], name: string): boolean {
	return saved.some((search) => search.name === name.trim());
}
