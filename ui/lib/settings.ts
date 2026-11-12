/**
 * Translation between the persisted settings file and the state the explorer
 * runs on. The file is editable by hand and written by other versions, so every
 * value that comes back is checked rather than trusted.
 */
import { availableColumns, defaultColumnIds, type ColumnId } from "./columns";
import { parseKeySequence } from "./keys";
import type { Keybinding } from "./keymap";
import { defaultPaneWidths, paneWidthFrom, type PaneWidths } from "./panes";
import { sidebarSectionsFrom, type SidebarSectionId } from "./sidebar";
import { defaultSortOrder, type SortKey, type SortOrder } from "./sorting";
import { whenClauseIsValid } from "./whenClause";

export type ViewMode = "rows" | "grid" | "columns";

/** The parts of the settings file the frontend owns. */
export type ViewSettings = {
	viewMode: ViewMode;
	columns: ColumnId[];
	showHidden: boolean;
	sort: SortOrder;
	lastDirectory: string;
	/** Widths of the panes flanking the listing. */
	panes: PaneWidths;
	/** The sidebar groups that are folded shut. */
	collapsedSections: SidebarSectionId[];
	/**
	 * Whether the details pane is on screen. It stays where it is either way:
	 * a pane that came and went with the selection moved the listing under the
	 * pointer every time the selection changed.
	 */
	showDetails: boolean;
	/** Appended after the built-in keymap, so these override it. */
	keybindings: Keybinding[];
};

/**
 * The file as it arrives: every field is whatever was on disk, which is why none
 * of them are typed as the values the explorer runs on.
 */
export type StoredSettings = {
	viewMode?: string;
	columns?: string[];
	showHidden?: boolean;
	sort?: { key?: string; descending?: boolean; directoriesFirst?: boolean };
	lastDirectory?: string;
	panes?: { sidebar?: number; details?: number };
	collapsedSections?: string[];
	showDetails?: boolean;
	keybindings?: Array<{ key?: string; command?: string; when?: string }>;
};

type StoredKeybinding = NonNullable<StoredSettings["keybindings"]>[number];

const sortKeys: SortKey[] = [
	"name",
	"natural",
	"extension",
	"size",
	"modified",
	"created",
	"permissions",
	"random",
];

export const defaultViewSettings: ViewSettings = {
	viewMode: "rows",
	columns: defaultColumnIds,
	showHidden: false,
	sort: defaultSortOrder,
	lastDirectory: "",
	panes: defaultPaneWidths,
	collapsedSections: [],
	showDetails: true,
	keybindings: [],
};

/** Reads a stored file into state, replacing anything unusable with a default. */
export function viewSettingsFrom(stored: StoredSettings): ViewSettings {
	return {
		viewMode: viewModeFrom(stored.viewMode),
		columns: columnIdsFrom(stored.columns),
		showHidden: stored.showHidden === true,
		sort: sortOrderFrom(stored.sort),
		lastDirectory: pathFrom(stored.lastDirectory),
		panes: paneWidthsFrom(stored.panes),
		collapsedSections: sidebarSectionsFrom(stored.collapsedSections),
		showDetails: showDetailsFrom(stored.showDetails),
		keybindings: keybindingsFrom(stored.keybindings),
	};
}

// A file that says nothing about the details pane gets it, since a pane nobody
// asked to hide is the more useful of the two.
function showDetailsFrom(stored: boolean | undefined): boolean {
	if (typeof stored !== "boolean") {
		return defaultViewSettings.showDetails;
	}
	return stored;
}

function paneWidthsFrom(stored: StoredSettings["panes"]): PaneWidths {
	if (!stored) {
		return defaultPaneWidths;
	}
	return {
		sidebar: paneWidthFrom("left", stored.sidebar, defaultPaneWidths.sidebar),
		details: paneWidthFrom("right", stored.details, defaultPaneWidths.details),
	};
}

/**
 * The keymap is hand-edited, so a line with a key nobody can read or a clause
 * that does not parse is dropped on its own rather than taking the rest of the
 * file with it.
 */
function keybindingsFrom(stored: StoredSettings["keybindings"]): Keybinding[] {
	if (!Array.isArray(stored)) {
		return [];
	}
	return stored.filter(isUsableKeybinding).map(toKeybinding);
}

function isUsableKeybinding(stored: StoredKeybinding): boolean {
	if (typeof stored.key !== "string" || parseKeySequence(stored.key).length === 0) {
		return false;
	}
	if (typeof stored.command !== "string") {
		return false;
	}
	return whenClauseIsValid(whenFrom(stored.when));
}

// An empty command is how a line unbinds a built-in key rather than replacing it.
function toKeybinding(stored: StoredKeybinding): Keybinding {
	return {
		key: String(stored.key),
		command: String(stored.command),
		when: whenFrom(stored.when),
	};
}

function whenFrom(stored: string | undefined): string {
	if (typeof stored !== "string") {
		return "";
	}
	return stored;
}

function viewModeFrom(stored: string | undefined): ViewMode {
	if (stored === "grid" || stored === "columns") {
		return stored;
	}
	return "rows";
}

// An empty list means the file said nothing, which is different from a file that
// deliberately turned every column off.
function columnIdsFrom(stored: string[] | undefined): ColumnId[] {
	if (!stored) {
		return defaultViewSettings.columns;
	}
	return stored.filter(isColumnId);
}

function sortOrderFrom(stored: StoredSettings["sort"]): SortOrder {
	if (!stored) {
		return defaultSortOrder;
	}
	return {
		key: sortKeyFrom(stored.key),
		descending: stored.descending === true,
		directoriesFirst: stored.directoriesFirst !== false,
	};
}

function sortKeyFrom(stored: string | undefined): SortKey {
	const known = sortKeys.find((key) => key === stored);
	if (!known) {
		return defaultSortOrder.key;
	}
	return known;
}

function pathFrom(stored: string | undefined): string {
	if (typeof stored !== "string") {
		return "";
	}
	return stored;
}

function isColumnId(value: string): value is ColumnId {
	return availableColumns.some((column) => column.id === value);
}
