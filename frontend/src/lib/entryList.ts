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
	/** Entries a cut is holding: dimmed until the paste lands. */
	cutPaths: Set<string>;
	focusIndex: number;
	renamingPath: string;
	onSelect: (index: number, event: MouseEvent) => void;
	onOpen: (entry: filesystem.Entry) => void;
	onContextMenu: (index: number, event: MouseEvent) => void;
	onRenameCommit: (name: string) => void;
	onRenameCancel: () => void;
};

