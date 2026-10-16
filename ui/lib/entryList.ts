import type { filesystem } from "../backend/models";
import type { ColumnDefinition } from "./columns";
import type { ListMetrics } from "./virtualList";

/** Ties an option element to aria-activedescendant on the list container. */
export function entryElementId(index: number): string {
	return `entry-option-${index}`;
}

/**
 * Everything a row or a tile needs to render one entry. Shared so the two views
 * cannot drift apart in what they support.
 *
 * `entries` is the mounted window rather than the whole directory: `firstIndex`
 * is where it starts in the listing, and the paddings stand in for the entries
 * above and below it.
 */
export type EntryViewProps = {
	entries: filesystem.Entry[];
	firstIndex: number;
	topPadding: number;
	bottomPadding: number;
	columns: ColumnDefinition[];
	selectedPaths: Set<string>;
	/** Entries a cut is holding: dimmed until the paste lands. */
	cutPaths: Set<string>;
	/** What the working tree says about each entry, keyed by name. */
	gitStatuses: Record<string, string>;
	focusIndex: number;
	renamingPath: string;
	/** The folder a drag is hovering over, or "" while none is. */
	dropTargetPath: string;
	onSelect: (index: number, event: MouseEvent) => void;
	onOpen: (entry: filesystem.Entry) => void;
	onContextMenu: (index: number, event: MouseEvent) => void;
	onDragStart: (index: number, event: DragEvent) => void;
	onRenameCommit: (name: string) => void;
	onRenameCancel: () => void;
	/** Reports the view's own layout, which drives the window it is handed. */
	onMetrics: (metrics: ListMetrics) => void;
};

/** Reads the absolute entry index a mounted option carries. */
export function indexOfOption(option: Element): number {
	const value = option.getAttribute("data-index");
	if (value === null) {
		return -1;
	}
	return Number(value);
}

