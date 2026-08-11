/**
 * Icons for the built-in commands, kept apart from the command data itself.
 * phosphor-svelte publishes its icons under an export condition only the Svelte
 * build resolves, so a module that imports one cannot be loaded by anything
 * else — including the tests that check the command set.
 */
import type { IconComponent } from "@neoworks-dev/ui";
import ArrowCounterClockwiseIcon from "phosphor-svelte/lib/ArrowCounterClockwiseIcon";
import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
import BookmarkSimpleIcon from "phosphor-svelte/lib/BookmarkSimpleIcon";
import ClipboardIcon from "phosphor-svelte/lib/ClipboardIcon";
import CopyIcon from "phosphor-svelte/lib/CopyIcon";
import FileIcon from "phosphor-svelte/lib/FileIcon";
import FolderPlusIcon from "phosphor-svelte/lib/FolderPlusIcon";
import PencilSimpleIcon from "phosphor-svelte/lib/PencilSimpleIcon";
import ScissorsIcon from "phosphor-svelte/lib/ScissorsIcon";
import TagIcon from "phosphor-svelte/lib/TagIcon";
import TrashIcon from "phosphor-svelte/lib/TrashIcon";
import XCircleIcon from "phosphor-svelte/lib/XCircleIcon";

const icons = new Map<string, IconComponent>([
	["navigate.open", ArrowSquareOutIcon],
	["navigate.openWith", ArrowSquareOutIcon],
	["file.cut", ScissorsIcon],
	["file.copy", CopyIcon],
	["file.paste", ClipboardIcon],
	["file.rename", PencilSimpleIcon],
	["file.trash", TrashIcon],
	["file.deletePermanently", XCircleIcon],
	["file.newFolder", FolderPlusIcon],
	["file.newFile", FileIcon],
	["file.undo", ArrowCounterClockwiseIcon],
	["tags.edit", TagIcon],
	["bookmark.pin", BookmarkSimpleIcon],
	["bookmark.unpin", XCircleIcon],
]);

/** A contributed command carries its own icon, so this is only the fallback. */
export function iconForCommand(id: string): IconComponent | undefined {
	return icons.get(id);
}
