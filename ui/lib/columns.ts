import type { filesystem } from "../backend/models";
import { classifyEntry, kindLabel } from "./fileKind";
import { formatFileSize, formatModifiedTime } from "./format";
import type { SortKey } from "./sorting";

export type ColumnId = "kind" | "size" | "modified" | "permissions";

export type ColumnDefinition = {
	id: ColumnId;
	label: string;
	/** Fixed track width so the header and every row line up. */
	widthClass: string;
	/** Extra classes for the value cell, e.g. a monospace mode string. */
	valueClass: string;
	valueOf: (entry: filesystem.Entry) => string;
	/**
	 * The key the Go side sorts this column by, empty where it cannot. Type is
	 * classified here from the file extension, and the backend does not have that
	 * table, so there is nothing for it to sort by.
	 */
	sortKey: SortKey | "";
};

export const availableColumns: ColumnDefinition[] = [
	{
		id: "kind",
		label: "Type",
		widthClass: "w-20",
		valueClass: "",
		valueOf: (entry) => kindLabel(classifyEntry(entry)),
		sortKey: "",
	},
	{
		id: "size",
		label: "Size",
		widthClass: "w-24",
		valueClass: "text-right tabular-nums",
		valueOf: sizeValue,
		sortKey: "size",
	},
	{
		id: "modified",
		label: "Modified",
		widthClass: "w-28",
		valueClass: "",
		valueOf: (entry) => formatModifiedTime(entry.modTime),
		sortKey: "modified",
	},
	{
		id: "permissions",
		label: "Permissions",
		widthClass: "w-28",
		valueClass: "font-mono",
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
