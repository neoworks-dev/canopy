import type { IconComponent } from "@neoworks-dev/ui";
import ArrowCounterClockwiseIcon from "phosphor-svelte/lib/ArrowCounterClockwiseIcon";
import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
import ClipboardIcon from "phosphor-svelte/lib/ClipboardIcon";
import CopyIcon from "phosphor-svelte/lib/CopyIcon";
import FileIcon from "phosphor-svelte/lib/FileIcon";
import FolderPlusIcon from "phosphor-svelte/lib/FolderPlusIcon";
import PencilSimpleIcon from "phosphor-svelte/lib/PencilSimpleIcon";
import ScissorsIcon from "phosphor-svelte/lib/ScissorsIcon";
import TrashIcon from "phosphor-svelte/lib/TrashIcon";
import XCircleIcon from "phosphor-svelte/lib/XCircleIcon";

/**
 * One registry of user actions, so the context menu, the keyboard bindings and
 * any later command palette cannot drift apart.
 */
export type ActionId =
	| "open"
	| "openWith"
	| "cut"
	| "copy"
	| "paste"
	| "rename"
	| "trash"
	| "deletePermanently"
	| "newFolder"
	| "newFile"
	| "undo";

export type ActionContext = {
	selectionCount: number;
	clipboardCount: number;
	canUndo: boolean;
};

export type ActionDefinition = {
	id: ActionId;
	label: string;
	shortcut: string;
	icon: IconComponent;
	isEnabled: (context: ActionContext) => boolean;
};

const definitions: ActionDefinition[] = [
	{
		id: "open",
		label: "Open",
		shortcut: "Enter",
		icon: ArrowSquareOutIcon,
		isEnabled: (context) => context.selectionCount > 0,
	},
	{
		id: "openWith",
		label: "Open with…",
		shortcut: "",
		icon: ArrowSquareOutIcon,
		isEnabled: (context) => context.selectionCount === 1,
	},
	{
		id: "cut",
		label: "Cut",
		shortcut: "Ctrl+X",
		icon: ScissorsIcon,
		isEnabled: (context) => context.selectionCount > 0,
	},
	{
		id: "copy",
		label: "Copy",
		shortcut: "Ctrl+C",
		icon: CopyIcon,
		isEnabled: (context) => context.selectionCount > 0,
	},
	{
		id: "paste",
		label: "Paste",
		shortcut: "Ctrl+V",
		icon: ClipboardIcon,
		isEnabled: (context) => context.clipboardCount > 0,
	},
	{
		id: "rename",
		label: "Rename",
		shortcut: "F2",
		icon: PencilSimpleIcon,
		isEnabled: (context) => context.selectionCount === 1,
	},
	{
		id: "trash",
		label: "Move to Trash",
		shortcut: "Delete",
		icon: TrashIcon,
		isEnabled: (context) => context.selectionCount > 0,
	},
	{
		id: "deletePermanently",
		label: "Delete permanently",
		shortcut: "Shift+Delete",
		icon: XCircleIcon,
		isEnabled: (context) => context.selectionCount > 0,
	},
	{
		id: "newFolder",
		label: "New folder",
		shortcut: "Ctrl+Shift+N",
		icon: FolderPlusIcon,
		isEnabled: () => true,
	},
	{
		id: "newFile",
		label: "New file",
		shortcut: "",
		icon: FileIcon,
		isEnabled: () => true,
	},
	{
		id: "undo",
		label: "Undo",
		shortcut: "Ctrl+Z",
		icon: ArrowCounterClockwiseIcon,
		isEnabled: (context) => context.canUndo,
	},
];

/** Menu layout: each inner array is one group, rendered with a separator. */
const selectionLayout: ActionId[][] = [
	["open", "openWith"],
	["cut", "copy", "paste"],
	["rename"],
	["trash", "deletePermanently"],
];

const backgroundLayout: ActionId[][] = [
	["newFolder", "newFile"],
	["paste"],
	["undo"],
];

export function selectionMenuGroups(): ActionDefinition[][] {
	return groupsFor(selectionLayout);
}

export function backgroundMenuGroups(): ActionDefinition[][] {
	return groupsFor(backgroundLayout);
}

export function actionById(id: ActionId): ActionDefinition {
	const definition = definitions.find((candidate) => candidate.id === id);
	if (!definition) {
		throw new Error(`unknown action ${id}`);
	}
	return definition;
}

function groupsFor(layout: ActionId[][]): ActionDefinition[][] {
	return layout.map((group) => group.map(actionById));
}
