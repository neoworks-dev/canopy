import type { filesystem } from "../backend/models";
import { classifyEntry, kindLabel } from "./fileKind";
import { formatFileSize, formatModifiedTime } from "./format";
import type { SortKey } from "./sorting";

export type ColumnId = "kind" | "size" | "modified" | "permissions";

export type ColumnDefinition = {
	id: ColumnId;
	label: string;
	/** Fixed track width in pixels so the header and every row line up. */
	width: number;
	isRightAligned: boolean;
	/** A monospace mode string, say. */
	isMonospace: boolean;
	valueOf: (entry: filesystem.Entry) => string;
	/**
	 * The key the backend sorts this column by, empty where it cannot. Type is
	 * classified here from the file extension, and the backend does not have that
	 * table, so there is nothing for it to sort by.
	 */
	sortKey: SortKey | "";
};

export const availableColumns: ColumnDefinition[] = [
	{
		id: "kind",
		label: "Type",
		width: 80,
		isRightAligned: false,
		isMonospace: false,
		valueOf: (entry) => kindLabel(classifyEntry(entry)),
		sortKey: "",
	},
	{
		id: "size",
		label: "Size",
		width: 96,
		isRightAligned: true,
		isMonospace: false,
		valueOf: sizeValue,
		sortKey: "size",
	},
	{
		id: "modified",
		label: "Modified",
		width: 112,
		isRightAligned: false,
		isMonospace: false,
		valueOf: (entry) => formatModifiedTime(entry.modTime),
		sortKey: "modified",
	},
	{
		id: "permissions",
		label: "Permissions",
		width: 112,
		isRightAligned: false,
		isMonospace: true,
		valueOf: permissionsValue,
		sortKey: "permissions",
	},
];

export const defaultColumnIds: ColumnId[] = ["size", "modified"];

export function columnsFor(visibleIds: ColumnId[]): ColumnDefinition[] {
	return availableColumns.filter((column) => visibleIds.includes(column.id));
}

/** The values a grid tile shows under the name, since it has no column tracks. */
export function columnSummary(entry: filesystem.Entry, columns: ColumnDefinition[]): string {
	const values = columns.map((column) => column.valueOf(entry));
	return values.join(" · ");
}

function sizeValue(entry: filesystem.Entry): string {
	if (entry.isDir) {
		return "—";
	}
	return formatFileSize(entry.size);
}

function permissionsValue(entry: filesystem.Entry): string {
	if (!entry.mode) {
		return "—";
	}
	return entry.mode;
}
