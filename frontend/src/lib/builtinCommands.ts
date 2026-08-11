/**
 * The explorer's own commands and the keymap that reaches them. Both are plain
 * data: a plugin contributes the same shapes, and the user's settings append to
 * the same binding list, so nothing here is privileged beyond coming first.
 *
 * Clause vocabulary, supplied by App.svelte on every key press:
 *   listActive     the directory listing is the pane on screen
 *   searchOpen     the location field is showing search rather than the path
 *   trashOpen      the trash pane has replaced the listing
 *   findOpen       the find-by-name prompt is showing
 *   selectionCount clipboardCount entryCount
 *   canUndo canGoBack canGoForward canGoUp
 *   viewMode       "rows" or "grid"
 */
import type { CommandDescriptor } from "./commands";
import type { Keybinding } from "./keymap";
import type { Movement } from "./movement";

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
	["first", "to the first entry"],
	["last", "to the last entry"],
];

/** Each movement's plain keys and the keys that extend the selection with it. */
const movementKeys: Array<[Movement, string[], string[]]> = [
	["down", ["down", "j"], ["shift+down", "J"]],
	["up", ["up", "k"], ["shift+up", "K"]],
	["left", ["left"], ["shift+left"]],
	["right", ["right"], ["shift+right"]],
	["pageDown", ["pagedown", "ctrl+d"], ["shift+pagedown"]],
	["pageUp", ["pageup", "ctrl+u"], ["shift+pageup"]],
	["first", ["home", "g g"], ["shift+home"]],
	["last", ["end", "G"], ["shift+end"]],
];

export const movementCommands: MovementCommand[] = buildMovementCommands();

/** Commands whose behaviour does not vary along an axis, listed by hand. */
export const staticCommandDescriptors: CommandDescriptor[] = [
	{ id: "navigate.open", title: "Open", category: "Navigation", when: "selectionCount > 0" },
	{
		id: "navigate.openWith",
		title: "Open with…",
		category: "Navigation",
		when: "selectionCount == 1",
	},
	{ id: "navigate.parent", title: "Go to parent folder", category: "Navigation", when: "canGoUp" },
	{ id: "navigate.back", title: "Back", category: "Navigation", when: "canGoBack" },
	{ id: "navigate.forward", title: "Forward", category: "Navigation", when: "canGoForward" },
	{ id: "navigate.home", title: "Go to home folder", category: "Navigation" },
	{ id: "navigate.trash", title: "Open trash", category: "Navigation" },
	{ id: "navigate.editPath", title: "Edit location", category: "Navigation" },

	{ id: "list.selectAll", title: "Select all", category: "Listing", when: "listActive" },
	{ id: "list.clearSelection", title: "Clear selection", category: "Listing" },
	{
		id: "list.toggleSelection",
		title: "Add entry to selection",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},
	{
		id: "list.find",
		title: "Find by name",
		category: "Listing",
		when: "listActive && entryCount > 0",
	},

	{ id: "view.rows", title: "Show as list", category: "View" },
	{ id: "view.grid", title: "Show as grid", category: "View" },
	{ id: "view.toggleHidden", title: "Show hidden files", category: "View" },
	{ id: "view.sortByName", title: "Sort by name", category: "View" },
	{ id: "view.sortBySize", title: "Sort by size", category: "View" },
	{ id: "view.sortByModified", title: "Sort by date modified", category: "View" },
	{ id: "view.sortByPermissions", title: "Sort by permissions", category: "View" },
	{ id: "view.reverseSort", title: "Reverse the sort order", category: "View" },
	{ id: "view.toggleDirectoriesFirst", title: "Keep folders first", category: "View" },

	{ id: "file.cut", title: "Cut", category: "File", when: "selectionCount > 0" },
	{ id: "file.copy", title: "Copy", category: "File", when: "selectionCount > 0" },
	{ id: "file.paste", title: "Paste", category: "File", when: "clipboardCount > 0" },
	{ id: "file.rename", title: "Rename", category: "File", when: "selectionCount == 1" },
	{ id: "file.trash", title: "Move to Trash", category: "File", when: "selectionCount > 0" },
	{
		id: "file.deletePermanently",
		title: "Delete permanently",
		category: "File",
		when: "selectionCount > 0",
	},
	{ id: "file.newFolder", title: "New folder", category: "File" },
	{ id: "file.newFile", title: "New file", category: "File" },
	{ id: "file.undo", title: "Undo", category: "File", when: "canUndo" },

	{ id: "search.here", title: "Search this folder", category: "Search" },
	{ id: "search.global", title: "Search everywhere", category: "Search" },
	{ id: "search.close", title: "Close search", category: "Search", when: "searchOpen" },
	{ id: "zoxide.jump", title: "Jump to a folder", category: "Navigation" },
];

/** Context menu layout: each inner array is one group, rendered with a rule. */
export const selectionMenuIds: string[][] = [
	["navigate.open", "navigate.openWith"],
	["file.cut", "file.copy", "file.paste"],
	["file.rename"],
	["file.trash", "file.deletePermanently"],
];

export const backgroundMenuIds: string[][] = [
	["file.newFolder", "file.newFile"],
	["file.paste"],
	["file.undo"],
];

/**
 * The listing is modal: with nothing focused for typing, a bare letter is a
 * command rather than the start of a name. Finding an entry by typing lives
 * behind "/" instead, which is what frees the letters up.
 *
 * The layout follows ranger, since that is the reference a keyboard-driven file
 * manager is measured against: hjkl to move and enter, HL for history, and the
 * doubled yy/dd/pp for the clipboard.
 */
export const defaultKeymap: Keybinding[] = [
	...movementBindings(),

	{ key: "space", command: "list.toggleSelection", when: "listActive" },
	{ key: "ctrl+a", command: "list.selectAll", when: "listActive" },
	{ key: "escape", command: "list.clearSelection", when: "listActive" },
	{ key: "/", command: "list.find", when: "listActive" },

	{ key: "enter", command: "navigate.open" },
	{ key: "l", command: "navigate.open", when: "listActive" },
	{ key: "i", command: "navigate.open", when: "listActive" },
	{ key: "o", command: "navigate.openWith", when: "listActive" },
	{ key: "h", command: "navigate.parent", when: "listActive" },
	{ key: "backspace", command: "navigate.parent" },
	{ key: "H", command: "navigate.back", when: "listActive" },
	{ key: "L", command: "navigate.forward", when: "listActive" },
	{ key: "alt+left", command: "navigate.back" },
	{ key: "alt+right", command: "navigate.forward" },
	{ key: "g h", command: "navigate.home" },
	{ key: "g t", command: "navigate.trash" },
	{ key: "ctrl+l", command: "navigate.editPath" },

	{ key: "y y", command: "file.copy", when: "listActive" },
	{ key: "d d", command: "file.cut", when: "listActive" },
	{ key: "p p", command: "file.paste", when: "listActive" },
	{ key: "ctrl+c", command: "file.copy" },
	{ key: "ctrl+x", command: "file.cut" },
	{ key: "ctrl+v", command: "file.paste" },
	{ key: "r", command: "file.rename", when: "listActive" },
	{ key: "f2", command: "file.rename" },
	{ key: "delete", command: "file.trash" },
	{ key: "d D", command: "file.trash", when: "listActive" },
	{ key: "shift+delete", command: "file.deletePermanently" },
	{ key: "n d", command: "file.newFolder", when: "listActive" },
	{ key: "n f", command: "file.newFile", when: "listActive" },
	{ key: "ctrl+shift+n", command: "file.newFolder" },
	{ key: "u", command: "file.undo", when: "listActive" },
	{ key: "ctrl+z", command: "file.undo" },

	{ key: "ctrl+1", command: "view.rows" },
	{ key: "ctrl+2", command: "view.grid" },
	{ key: ".", command: "view.toggleHidden", when: "listActive" },
	{ key: "ctrl+h", command: "view.toggleHidden" },
	{ key: "s n", command: "view.sortByName", when: "listActive" },
	{ key: "s s", command: "view.sortBySize", when: "listActive" },
	{ key: "s m", command: "view.sortByModified", when: "listActive" },
	{ key: "s p", command: "view.sortByPermissions", when: "listActive" },
	{ key: "s r", command: "view.reverseSort", when: "listActive" },
	{ key: "s d", command: "view.toggleDirectoriesFirst", when: "listActive" },

	{ key: "f", command: "search.here", when: "listActive" },
	{ key: "z", command: "search.global", when: "listActive" },
	{ key: "ctrl+f", command: "search.here" },
	{ key: "Z", command: "zoxide.jump", when: "listActive" },
	{ key: "ctrl+j", command: "zoxide.jump" },
	{ key: "escape", command: "search.close", when: "searchOpen" },
];

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
	for (const [movement, plain, extending] of movementKeys) {
		appendBindings(bindings, plain, `list.move.${movement}`);
		appendBindings(bindings, extending, `list.extend.${movement}`);
	}
	return bindings;
}

function appendBindings(bindings: Keybinding[], keys: string[], command: string) {
	for (const key of keys) {
		bindings.push({ key, command, when: "listActive" });
	}
}
