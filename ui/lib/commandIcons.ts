/**
 * Icons for the built-in commands, as SVG markup, kept apart from the command
 * data itself so the command set can be tested without loading icon files.
 */
import {
	arrowClockwise,
	arrowCounterClockwise,
	arrowSquareOut,
	bookmarkSimple,
	clipboard,
	copy,
	file,
	fileArchive,
	folderPlus,
	pencilSimple,
	scissors,
	tag,
	terminalWindow,
	trash,
	xCircle,
} from "../icons";

const icons = new Map<string, string>([
	["navigate.open", arrowSquareOut],
	["navigate.openWith", arrowSquareOut],
	["navigate.terminal", terminalWindow],
	["file.cut", scissors],
	["file.copy", copy],
	["file.paste", clipboard],
	["file.rename", pencilSimple],
	["file.trash", trash],
	["file.deletePermanently", xCircle],
	["file.extract", fileArchive],
	["file.compress", fileArchive],
	["file.newFolder", folderPlus],
	["file.newFile", file],
	["file.undo", arrowCounterClockwise],
	["file.redo", arrowClockwise],
	["tags.edit", tag],
	["bookmark.pin", bookmarkSimple],
	["bookmark.unpin", xCircle],
]);

/** A contributed command carries its own icon, so this is only the fallback. */
export function iconForCommand(id: string): string | undefined {
	return icons.get(id);
}
