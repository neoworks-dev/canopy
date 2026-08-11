import type { filesystem } from "../../wailsjs/go/models";
import type { ColumnDefinition } from "./columns";

/** Ties an option element to aria-activedescendant on the list container. */
export function entryElementId(index: number): string {
	return `entry-option-${index}`;
}

/**
 * Everything a row or a tile needs to render one entry. Shared so the two views
 * cannot drift apart in what they support.
 */
export type EntryViewProps = {
	entries: filesystem.Entry[];
	columns: ColumnDefinition[];
	selectedPaths: Set<string>;
	focusIndex: number;
	renamingPath: string;
	onSelect: (index: number, event: MouseEvent) => void;
	onOpen: (entry: filesystem.Entry) => void;
	onContextMenu: (index: number, event: MouseEvent) => void;
	onRenameCommit: (name: string) => void;
	onRenameCancel: () => void;
};

/**
 * Selection paints a background, keyboard focus paints a ring, so the two states
 * stay distinguishable when they land on different entries.
 */
export function entryStateClasses(
	entry: filesystem.Entry,
	index: number,
	selectedPaths: Set<string>,
	focusIndex: number,
): string {
	const classes: string[] = [];
	if (selectedPaths.has(entry.path)) {
		classes.push("bg-raised");
	}
	if (!selectedPaths.has(entry.path)) {
		classes.push("hover:bg-hover");
	}
	if (index === focusIndex) {
		classes.push("ring-2 ring-inset ring-line-strong");
	}
	return classes.join(" ");
}
