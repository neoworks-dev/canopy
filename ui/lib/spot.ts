/**
 * The lines Yazi's spot shows for the hovered entry: what it is, where it is,
 * how big, when, and whose.
 */
import type { filesystem } from "../backend/models";
import { classifyEntry, kindLabel } from "./fileKind";
import { formatFileSize, formatModifiedTime } from "./format";

export type SpotLine = { label: string; value: string };

export function spotLines(entry: filesystem.Entry): SpotLine[] {
	return [
		{ label: "Name", value: entry.name },
		{ label: "Path", value: entry.path },
		{ label: "Type", value: kindLabel(classifyEntry(entry)) },
		...sizeLines(entry),
		...linkLines(entry),
		{ label: "Modified", value: formatModifiedTime(entry.modTime) },
		{ label: "Created", value: formatModifiedTime(entry.createdTime) },
		{ label: "Permissions", value: entry.mode },
		{ label: "Owner", value: entry.owner },
	];
}

// A folder's own size is the size of its directory record, which says nothing.
function sizeLines(entry: filesystem.Entry): SpotLine[] {
	if (entry.isDir) {
		return [];
	}
	return [{ label: "Size", value: `${formatFileSize(entry.size)} (${entry.size.toLocaleString()} bytes)` }];
}

function linkLines(entry: filesystem.Entry): SpotLine[] {
	if (!entry.isSymlink) {
		return [];
	}
	return [{ label: "Links to", value: entry.linkTarget }];
}
