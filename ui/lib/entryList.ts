import type { PointerEvent } from "@neoworks/svelte-gpui/ui";
import type { filesystem } from "../backend/models";
import type { ColumnDefinition } from "./columns";
import type { GridLayout } from "./marquee";

/**
 * Everything a row or a tile needs to draw one entry and to report what the
 * user did with it. Shared so the views cannot drift apart in what they support.
 *
 * `entries` is the whole of what the directory shows: the list under each view
 * only builds the rows near the window.
 */
export type EntryViewProps = {
	entries: filesystem.Entry[];
	columns: ColumnDefinition[];
	selectedPaths: Set<string>;
	/** Entries a cut is holding: dimmed until the paste lands. */
	cutPaths: Set<string>;
	/** Entries a copy is holding: marked until something else is copied. */
	copiedPaths: Set<string>;
	/** What the working tree says about each entry, keyed by name. */
	gitStatuses: Record<string, string>;
	focusIndex: number;
	renamingPath: string;
	/** A press on an entry: it selects, and a second press opens it. */
	onPress: (index: number, event: PointerEvent) => void;
	/** What a drag that starts on this entry carries. */
	dragPathsFor: (entry: filesystem.Entry) => string[];
	onDragStart: (index: number) => void;
	onDragEnd: () => void;
	/** Something dragged out of this window was dropped on a folder entry. */
	onDropOnFolder: (folder: filesystem.Entry, sources: string[]) => void;
	/** Whether dropping these paths into this folder would do anything, which is what lights it up. */
	canDropInto: (folder: string, sources: string[]) => boolean;
	onRenameCommit: (name: string) => void;
	onRenameCancel: () => void;
	/** Reports where the entries sit, which is what a band and a right press are matched against. */
	onLayout: (layout: GridLayout) => void;
	/** Reports how far the entries are scrolled. */
	onScroll: (scrollTop: number) => void;
};
