import { invoke } from "@neoworks/svelte-gpui";
import type { filesystem, proto } from "./models";

export function IndexStatus(): Promise<proto.StatusResult> {
	return invoke<proto.StatusResult>("IndexStatus");
}

export function Search(query: string, scope: string): Promise<proto.SearchResult> {
	return invoke<proto.SearchResult>("Search", { query, scope });
}

export function SearchEntries(query: string, scope: string): Promise<Array<filesystem.Entry>> {
	return invoke<Array<filesystem.Entry>>("SearchEntries", { query, scope });
}
