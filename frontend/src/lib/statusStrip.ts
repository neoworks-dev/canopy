/**
 * What the strip under the listing says. Kept free of the DOM: the wording is
 * the part worth checking, and the strip itself is two spans.
 */
import type { filesystem } from "../../wailsjs/go/models";
import { classifyEntry, kindLabel } from "./fileKind";
import { formatFileSize, formatModifiedTime } from "./format";

export type StatusSummary = {
	/** The sentence, always present while a listing is on screen. */
	primary: string;
	/** The detail after it, empty when there is nothing worth adding. */
	secondary: string;
};

export type StatusInput = {
	entryCount: number;
	/** Entries the current filter is keeping off screen. */
	hiddenCount: number;
	selected: filesystem.Entry[];
};

export const emptyStatus: StatusSummary = { primary: "", secondary: "" };

export function summarize(input: StatusInput): StatusSummary {
	if (input.selected.length === 1) {
		return describeOne(input.selected[0]);
	}
	if (input.selected.length > 1) {
		return describeSeveral(input.selected, input.entryCount);
	}
	return describeFolder(input.entryCount, input.hiddenCount);
}

function describeOne(entry: filesystem.Entry): StatusSummary {
	return { primary: entry.name, secondary: detailsOf(entry) };
}

function detailsOf(entry: filesystem.Entry): string {
	const parts = [kindLabel(classifyEntry(entry)), formatModifiedTime(entry.modTime)];
	if (entry.isDir) {
		return parts.join(" · ");
	}
	return [parts[0], formatFileSize(entry.size), parts[1]].join(" · ");
}

function describeSeveral(selected: filesystem.Entry[], entryCount: number): StatusSummary {
	const bytes = selected.reduce((total, entry) => total + sizeOf(entry), 0);
	return {
		primary: `${selected.length} of ${itemCount(entryCount)} selected`,
		secondary: formatFileSize(bytes),
	};
}

// Folders report the size of their own record rather than of what is inside
// them, which would make a selection total meaningless.
function sizeOf(entry: filesystem.Entry): number {
	if (entry.isDir) {
		return 0;
	}
	return entry.size;
}

function describeFolder(entryCount: number, hiddenCount: number): StatusSummary {
	return { primary: itemCount(entryCount), secondary: hiddenLabel(hiddenCount) };
}

function hiddenLabel(hiddenCount: number): string {
	if (hiddenCount <= 0) {
		return "";
	}
	return `${hiddenCount} hidden`;
}

export function itemCount(count: number): string {
	if (count === 1) {
		return "1 item";
	}
	return `${count} items`;
}
