/**
 * The explorer's own commands and the keymap that reaches them. Both are plain
 * data: a plugin contributes the same shapes, and the user's settings append to
 * the same binding list, so nothing here is privileged beyond coming first.
 *
 * Clause vocabulary, supplied by App.svelte on every key press:
 *   listActive     the directory listing is the pane in focus
 *   searchOpen     the location field is showing search rather than the path
 *   trashOpen      the trash pane has replaced the listing
 *   selectionCount clipboardCount entryCount
 *   canUndo canRedo canGoBack canGoForward canGoUp
 *   selectionIsArchive the one selected entry is an archive that can be opened
 *   viewMode       "rows", "grid" or "columns"
 *   folderPinned   the folder in focus is pinned to the sidebar
 *   directoryOpen  the pane is showing a directory rather than a virtual list
 *   paletteOpen    the command palette is on screen
 *   tabCount paneCount canSplit
 */
import type { ColumnId } from "./columns";
import type { CommandDescriptor } from "./commands";
import type { CopyTextKind } from "./copyText";
import type { Keybinding } from "./keymap";
import type { Movement } from "./movement";
import type { SortKey } from "./sorting";

export type MovementCommand = {
	id: string;
	title: string;
	movement: Movement;
	/** Extending carries the selection along instead of replacing it. */
	extend: boolean;
};

const movementTitles: Array<[Movement, string]> = [
	["down", "down"],
	["up", "up"],
	["left", "left"],
	["right", "right"],
	["pageDown", "down a page"],
	["pageUp", "up a page"],
	["halfPageDown", "down half a page"],
	["halfPageUp", "up half a page"],
	["first", "to the first entry"],
	["last", "to the last entry"],
];

type MovementKeys = {
	movement: Movement;
	/** Keys that move the cursor. */
	plain: string[];
	/** Keys that carry the selection along with the move. */
	extending: string[];
	when: string;
};

// Left and right only move the cursor where a row holds several entries; in the
// other views they go up a folder and into one.
const movementKeys: MovementKeys[] = [
	{ movement: "down", plain: ["down", "j"], extending: ["shift+down"], when: "listActive" },
	{ movement: "up", plain: ["up", "k"], extending: ["shift+up"], when: "listActive" },
	{ movement: "left", plain: ["left"], extending: ["shift+left"], when: "listActive && viewMode == grid" },
	{ movement: "right", plain: ["right"], extending: ["shift+right"], when: "listActive && viewMode == grid" },
	{ movement: "pageDown", plain: ["pagedown", "ctrl+f"], extending: [], when: "listActive" },
	{ movement: "pageUp", plain: ["pageup", "ctrl+b"], extending: [], when: "listActive" },
	{ movement: "halfPageDown", plain: ["ctrl+d", "shift+pagedown"], extending: [], when: "listActive" },
	{ movement: "halfPageUp", plain: ["ctrl+u", "shift+pageup"], extending: [], when: "listActive" },
	{ movement: "first", plain: ["home", "g g"], extending: ["shift+home"], when: "listActive" },
	{ movement: "last", plain: ["end", "G"], extending: ["shift+end"], when: "listActive" },
];

export const movementCommands: MovementCommand[] = buildMovementCommands();

export type SortCommand = {
	id: string;
	title: string;
	key: SortKey;
	descending: boolean;
	/** Bound to the lower case key for ascending and the capital for descending. */
	binding: string;
	/** The linemode that shows what the order is by, as Yazi switches to it; "" leaves the columns alone. */
	linemodeId: string;
};

type SortKeyRow = { key: SortKey; label: string; letter: string; linemodeId: string };

const sortKeyTable: SortKeyRow[] = [
	{ key: "name", label: "name", letter: "a", linemodeId: "" },
	{ key: "modified", label: "date modified", letter: "m", linemodeId: "view.linemode.modified" },
	{ key: "size", label: "size", letter: "s", linemodeId: "view.linemode.size" },
	{ key: "permissions", label: "permissions", letter: "", linemodeId: "" },
];

export const sortCommands: SortCommand[] = buildSortCommands();

export type CopyTextCommand = {
	id: string;
	title: string;
	kind: CopyTextKind;
	binding: string;
};

export const copyTextCommands: CopyTextCommand[] = [
	{ id: "copy.path", title: "Copy the path", kind: "path", binding: "c c" },
	{ id: "copy.url", title: "Copy the file URL", kind: "url", binding: "c C" },
	{ id: "copy.dirpath", title: "Copy the folder's path", kind: "dirpath", binding: "c d" },
	{ id: "copy.dirurl", title: "Copy the folder's URL", kind: "dirurl", binding: "c D" },
	{ id: "copy.filename", title: "Copy the name", kind: "filename", binding: "c f" },
	{
		id: "copy.nameWithoutExtension",
		title: "Copy the name without its extension",
		kind: "nameWithoutExtension",
		binding: "c n",
	},
];

export type LinemodeCommand = {
	id: string;
	title: string;
	columns: ColumnId[];
	binding: string;
};

export const linemodeCommands: LinemodeCommand[] = [
	{ id: "view.linemode.size", title: "Show only the size", columns: ["size"], binding: "m s" },
	{
		id: "view.linemode.permissions",
		title: "Show only the permissions",
		columns: ["permissions"],
		binding: "m p",
	},
	{ id: "view.linemode.modified", title: "Show only the date modified", columns: ["modified"], binding: "m m" },
	{ id: "view.linemode.none", title: "Show no details", columns: [], binding: "m n" },
];

export const tabSelectCommands: Array<{ id: string; title: string; index: number }> = buildTabSelectCommands();

/** Commands whose behaviour does not vary along an axis, listed by hand. */
export const staticCommandDescriptors: CommandDescriptor[] = [
	{ id: "navigate.open", title: "Open", category: "Navigation", when: "selectionCount > 0" },
	{
		id: "navigate.openWith",
		title: "Open with…",
		category: "Navigation",
		when: "selectionCount == 1",
	},
	{ id: "navigate.enter", title: "Enter the folder under the cursor", category: "Navigation", when: "listActive" },
	{ id: "navigate.parent", title: "Go to parent folder", category: "Navigation", when: "canGoUp" },
	{ id: "navigate.back", title: "Back", category: "Navigation", when: "canGoBack" },
	{ id: "navigate.forward", title: "Forward", category: "Navigation", when: "canGoForward" },
	{ id: "navigate.home", title: "Go to home folder", category: "Navigation" },
	{ id: "navigate.config", title: "Go to ~/.config", category: "Navigation" },
	{ id: "navigate.downloads", title: "Go to the downloads folder", category: "Navigation" },
	{ id: "navigate.trash", title: "Open trash", category: "Navigation" },
	{
		id: "navigate.terminal",
		title: "Open terminal here",
		category: "Navigation",
		when: "directoryOpen",
	},
	{ id: "navigate.editPath", title: "Edit location", category: "Navigation" },

	{ id: "list.selectAll", title: "Select all", category: "Listing", when: "listActive" },
	{ id: "list.invertSelection", title: "Invert selection", category: "Listing", when: "listActive" },
	{
		id: "list.escape",
		title: "Leave visual mode, or clear the selection, filter and find",
		category: "Listing",
	},
	{
		id: "list.toggleSelection",
		title: "Select entry and move down",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{
		id: "list.visual",
		title: "Select a range as the cursor moves",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{
		id: "list.visualUnset",
		title: "Deselect a range as the cursor moves",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{ id: "list.filter", title: "Filter entries by name", category: "Listing", when: "listActive" },
	{
		id: "list.find",
		title: "Find forwards by name",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{
		id: "list.findBackward",
		title: "Find backwards by name",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{ id: "list.findNext", title: "Next match", category: "Listing", when: "listActive" },
	{ id: "list.findPrevious", title: "Previous match", category: "Listing", when: "listActive" },

	{
		id: "preview.seekDown",
		title: "Scroll the preview down",
		category: "View",
		when: "listActive && selectionCount == 1",
	},
	{
		id: "preview.seekUp",
		title: "Scroll the preview up",
		category: "View",
		when: "listActive && selectionCount == 1",
	},
	...copyTextCommands.map((command) => ({
		id: command.id,
		title: command.title,
		category: "File",
		when: "selectionCount > 0",
	})),

	{ id: "tab.new", title: "New tab", category: "Tabs" },
	{ id: "tab.rename", title: "Rename tab", category: "Tabs" },
	{
		id: "tab.close",
		title: "Close tab",
		category: "Tabs",
		when: "tabCount > 1 || paneCount > 1",
	},
	{ id: "tab.next", title: "Next tab", category: "Tabs", when: "tabCount > 1" },
	{ id: "tab.previous", title: "Previous tab", category: "Tabs", when: "tabCount > 1" },
	{ id: "tab.swapNext", title: "Move tab right", category: "Tabs", when: "tabCount > 1" },
	{ id: "tab.swapPrevious", title: "Move tab left", category: "Tabs", when: "tabCount > 1" },
	...tabSelectCommands.map((command) => ({
		id: command.id,
		title: command.title,
		category: "Tabs",
		when: `tabCount > ${command.index}`,
	})),
	{ id: "pane.split", title: "Split the view", category: "Tabs", when: "canSplit" },
	{ id: "pane.close", title: "Close this pane", category: "Tabs", when: "paneCount > 1" },
	{
		id: "pane.focusOther",
		title: "Focus the other pane",
		category: "Tabs",
		when: "paneCount > 1",
	},

	{ id: "view.rows", title: "Show as list", category: "View" },
	{ id: "view.grid", title: "Show as grid", category: "View" },
	{ id: "view.columns", title: "Show as columns", category: "View" },
	{ id: "view.toggleHidden", title: "Show hidden files", category: "View" },
	{ id: "view.toggleDetails", title: "Show the details pane", category: "View" },
	{ id: "view.reverseSort", title: "Reverse the sort order", category: "View" },
	...sortCommands.map((command) => ({ id: command.id, title: command.title, category: "View" })),
	...linemodeCommands.map((command) => ({ id: command.id, title: command.title, category: "View" })),
	{ id: "view.toggleDirectoriesFirst", title: "Keep folders first", category: "View" },

	{ id: "file.cut", title: "Cut", category: "File", when: "selectionCount > 0" },
	{ id: "file.copy", title: "Copy", category: "File", when: "selectionCount > 0" },
	{ id: "file.paste", title: "Paste", category: "File", when: "clipboardCount > 0" },
	{
		id: "file.pasteOverwrite",
		title: "Paste, replacing what is there",
		category: "File",
		when: "clipboardCount > 0",
	},
	{ id: "file.cancelYank", title: "Forget what was copied or cut", category: "File", when: "clipboardCount > 0" },
	{ id: "file.create", title: "Create a file or folder…", category: "File", when: "directoryOpen" },
	{ id: "file.bulkCreate", title: "Create several files or folders…", category: "File", when: "directoryOpen" },
	{ id: "file.rename", title: "Rename", category: "File", when: "selectionCount > 0" },
	{ id: "file.trash", title: "Move to Trash", category: "File", when: "selectionCount > 0" },
	{
		id: "file.deletePermanently",
		title: "Delete permanently",
		category: "File",
		when: "selectionCount > 0",
	},
	{
		id: "file.extract",
		title: "Extract here",
		category: "File",
		when: "selectionIsArchive",
	},
	{ id: "file.compress", title: "Compress", category: "File", when: "selectionCount > 0" },
	{ id: "file.newFolder", title: "New folder", category: "File" },
	{ id: "file.newFile", title: "New file", category: "File" },
	{ id: "file.undo", title: "Undo", category: "File", when: "canUndo" },
	{ id: "file.redo", title: "Redo", category: "File", when: "canRedo" },
	{ id: "tags.edit", title: "Tags…", category: "File", when: "selectionCount > 0" },

	{
		id: "bookmark.pin",
		title: "Pin this folder",
		category: "Navigation",
		when: "listActive && !folderPinned",
	},
	{
		id: "bookmark.unpin",
		title: "Unpin this folder",
		category: "Navigation",
		when: "listActive && folderPinned",
	},

	{
		id: "search.duplicates",
		title: "Find duplicate files here",
		category: "Search",
		when: "directoryOpen",
	},
	{ id: "search.here", title: "Search this folder", category: "Search" },
	{ id: "search.global", title: "Search everywhere", category: "Search" },
	{ id: "search.close", title: "Close search", category: "Search", when: "searchOpen" },
	{ id: "search.save", title: "Save this search", category: "Search", when: "searchOpen" },
	{ id: "zoxide.jump", title: "Jump to a folder", category: "Navigation" },
	{
		id: "palette.open",
		title: "Show all commands",
		category: "General",
		when: "!paletteOpen",
	},
	{ id: "help.open", title: "Show every key", category: "General" },
	{ id: "tasks.show", title: "Show running tasks", category: "General" },
	{
		id: "list.spot",
		title: "Show everything about the entry under the cursor",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{ id: "app.quit", title: "Quit", category: "General" },
];

/** Context menu layout: each inner array is one group, rendered with a rule. */
export const selectionMenuIds: string[][] = [
	["navigate.open", "navigate.openWith"],
	["file.cut", "file.copy", "file.paste"],
	["file.rename", "tags.edit"],
	["file.extract", "file.compress"],
	["file.trash", "file.deletePermanently"],
];

export const backgroundMenuIds: string[][] = [
	["file.newFolder", "file.newFile"],
	["file.paste"],
	["navigate.terminal"],
	["bookmark.pin", "bookmark.unpin"],
	["file.undo", "file.redo"],
];

/**
 * The listing is modal: with nothing focused for typing, a bare letter is a
 * command rather than the start of a name.
 *
 * The layout is Yazi's, key for key, so the same hands work in both: hjkl to
 * move, `y x p d` for the clipboard, `f` to filter, `/` to find, `g …` to go
 * somewhere, `, …` to sort and `m …` to choose what is shown beside each name.
 * Chords with a modifier that Yazi leaves free keep the usual desktop meaning
 * (ctrl+c copies, ctrl+z undoes), because a window has no terminal to interrupt.
 */
export const defaultKeymap: Keybinding[] = [
	...movementBindings(),

	{ key: "space", command: "list.toggleSelection", when: "listActive" },
	{ key: "ctrl+a", command: "list.selectAll", when: "listActive" },
	{ key: "ctrl+r", command: "list.invertSelection", when: "listActive" },
	{ key: "v", command: "list.visual", when: "listActive" },
	{ key: "V", command: "list.visualUnset", when: "listActive" },
	{ key: "escape", command: "list.escape", when: "listActive" },
	{ key: "ctrl+[", command: "list.escape", when: "listActive" },
	{ key: "f", command: "list.filter", when: "listActive" },
	{ key: "/", command: "list.find", when: "listActive" },
	{ key: "?", command: "list.findBackward", when: "listActive" },
	{ key: "n", command: "list.findNext", when: "listActive" },
	{ key: "N", command: "list.findPrevious", when: "listActive" },

	{ key: "enter", command: "navigate.open" },
	{ key: "o", command: "navigate.open", when: "listActive" },
	{ key: "O", command: "navigate.openWith", when: "listActive" },
	{ key: "shift+enter", command: "navigate.openWith", when: "listActive" },
	{ key: "K", command: "preview.seekUp", when: "listActive" },
	{ key: "J", command: "preview.seekDown", when: "listActive" },
	{ key: "l", command: "navigate.enter", when: "listActive" },
	{ key: "right", command: "navigate.enter", when: "listActive && viewMode != grid" },
	{ key: "h", command: "navigate.parent", when: "listActive" },
	{ key: "left", command: "navigate.parent", when: "listActive && viewMode != grid" },
	{ key: "backspace", command: "navigate.parent" },
	{ key: "H", command: "navigate.back", when: "listActive" },
	{ key: "L", command: "navigate.forward", when: "listActive" },
	{ key: "alt+left", command: "navigate.back" },
	{ key: "alt+right", command: "navigate.forward" },
	{ key: "g h", command: "navigate.home" },
	{ key: "g c", command: "navigate.config" },
	{ key: "g d", command: "navigate.downloads" },
	{ key: "g t", command: "navigate.trash" },
	{ key: "g space", command: "zoxide.jump" },
	{ key: "g s", command: "navigate.terminal", when: "listActive" },
	{ key: "ctrl+alt+t", command: "navigate.terminal" },
	{ key: "ctrl+l", command: "navigate.editPath" },
	{ key: "tab", command: "list.spot", when: "listActive" },
	{ key: "w", command: "tasks.show", when: "listActive" },

	{ key: "y", command: "file.copy", when: "listActive" },
	{ key: "x", command: "file.cut", when: "listActive" },
	{ key: "p", command: "file.paste", when: "listActive" },
	{ key: "P", command: "file.pasteOverwrite", when: "listActive" },
	{ key: "Y", command: "file.cancelYank", when: "listActive" },
	{ key: "X", command: "file.cancelYank", when: "listActive" },
	{ key: "ctrl+c", command: "file.copy" },
	{ key: "ctrl+x", command: "file.cut" },
	{ key: "ctrl+v", command: "file.paste" },
	{ key: "d", command: "file.trash", when: "listActive" },
	{ key: "D", command: "file.deletePermanently", when: "listActive" },
	{ key: "delete", command: "file.trash" },
	{ key: "shift+delete", command: "file.deletePermanently" },
	{ key: "a", command: "file.create", when: "listActive" },
	{ key: "A", command: "file.bulkCreate", when: "listActive" },
	...copyTextBindings(),
	{ key: "r", command: "file.rename", when: "listActive" },
	{ key: "f2", command: "file.rename" },
	{ key: "ctrl+shift+n", command: "file.newFolder" },
	{ key: "T", command: "tags.edit", when: "listActive" },
	{ key: "b", command: "bookmark.pin", when: "listActive" },
	{ key: "u", command: "file.undo", when: "listActive" },
	{ key: "U", command: "file.redo", when: "listActive" },
	{ key: "ctrl+z", command: "file.undo" },
	{ key: "ctrl+shift+z", command: "file.redo" },
	{ key: "ctrl+y", command: "file.redo" },

	{ key: "t t", command: "tab.new", when: "listActive" },
	{ key: "t r", command: "tab.rename", when: "listActive" },
	{ key: "ctrl+t", command: "tab.new" },
	{ key: "ctrl+w", command: "tab.close" },
	{ key: "[", command: "tab.previous", when: "listActive" },
	{ key: "]", command: "tab.next", when: "listActive" },
	{ key: "{", command: "tab.swapPrevious", when: "listActive" },
	{ key: "}", command: "tab.swapNext", when: "listActive" },
	...tabSelectBindings(),
	{ key: "ctrl+tab", command: "tab.next" },
	{ key: "ctrl+shift+tab", command: "tab.previous" },
	{ key: "ctrl+\\", command: "pane.split" },
	{ key: "ctrl+shift+w", command: "pane.close" },
	{ key: "f6", command: "pane.focusOther" },

	{ key: "ctrl+1", command: "view.rows" },
	{ key: "ctrl+2", command: "view.grid" },
	{ key: "ctrl+3", command: "view.columns" },
	{ key: ".", command: "view.toggleHidden", when: "listActive" },
	{ key: "ctrl+h", command: "view.toggleHidden" },
	...sortBindings(),
	...linemodeBindings(),

	{ key: "s", command: "search.here", when: "listActive" },
	{ key: "z", command: "search.global", when: "listActive" },
	{ key: "Z", command: "zoxide.jump", when: "listActive" },
	{ key: "ctrl+j", command: "zoxide.jump" },
	{ key: "ctrl+s", command: "search.close", when: "searchOpen" },
	{ key: "escape", command: "search.close", when: "searchOpen" },
	{ key: "ctrl+shift+s", command: "search.save", when: "searchOpen" },

	{ key: "ctrl+shift+p", command: "palette.open" },
	{ key: ":", command: "palette.open", when: "listActive" },
	{ key: ";", command: "palette.open", when: "listActive" },
	{ key: "~", command: "help.open", when: "listActive" },
	{ key: "f1", command: "help.open" },
	{ key: "q", command: "app.quit", when: "listActive" },
	{ key: "Q", command: "app.quit", when: "listActive" },
];

function sortBindings(): Keybinding[] {
	return sortCommands
		.filter((command) => command.binding !== "")
		.map((command) => ({ key: command.binding, command: command.id, when: "listActive" }));
}

function copyTextBindings(): Keybinding[] {
	return copyTextCommands.map((command) => ({
		key: command.binding,
		command: command.id,
		when: "listActive",
	}));
}

function linemodeBindings(): Keybinding[] {
	return linemodeCommands.map((command) => ({
		key: command.binding,
		command: command.id,
		when: "listActive",
	}));
}

function tabSelectBindings(): Keybinding[] {
	return tabSelectCommands.map((command) => ({
		key: String(command.index + 1),
		command: command.id,
		when: "listActive",
	}));
}

function buildSortCommands(): SortCommand[] {
	const commands: SortCommand[] = [];
	for (const row of sortKeyTable) {
		commands.push(sortCommand(row, false, row.letter));
		commands.push(sortCommand(row, true, row.letter.toUpperCase()));
	}
	return commands;
}

function sortCommand(row: SortKeyRow, descending: boolean, letter: string): SortCommand {
	let direction = "ascending";
	if (descending) {
		direction = "descending";
	}
	let binding = "";
	if (letter !== "") {
		binding = `, ${letter}`;
	}
	return {
		id: `view.sort.${row.key}.${direction}`,
		title: `Sort by ${row.label}, ${direction}`,
		key: row.key,
		descending,
		binding,
		linemodeId: row.linemodeId,
	};
}

function buildTabSelectCommands(): Array<{ id: string; title: string; index: number }> {
	const commands = [];
	for (let number = 1; number <= 9; number++) {
		commands.push({ id: `tab.select.${number}`, title: `Go to tab ${number}`, index: number - 1 });
	}
	return commands;
}

function buildMovementCommands(): MovementCommand[] {
	const commands: MovementCommand[] = [];
	for (const [movement, phrase] of movementTitles) {
		commands.push({ id: `list.move.${movement}`, title: `Move ${phrase}`, movement, extend: false });
		commands.push({
			id: `list.extend.${movement}`,
			title: `Extend selection ${phrase}`,
			movement,
			extend: true,
		});
	}
	return commands;
}

function movementBindings(): Keybinding[] {
	const bindings: Keybinding[] = [];
	for (const { movement, plain, extending, when } of movementKeys) {
		appendBindings(bindings, plain, `list.move.${movement}`, when);
		appendBindings(bindings, extending, `list.extend.${movement}`, when);
	}
	return bindings;
}

function appendBindings(bindings: Keybinding[], keys: string[], command: string, when: string) {
	for (const key of keys) {
		bindings.push({ key, command, when });
	}
}
