/**
 * How the states git reports are drawn. The Go side folds a folder's contents
 * into the folder itself, so anything here applies to a single entry.
 */
import { colors } from "../theme";

export type GitMark = {
	/** The letter shown beside the entry, empty where the colour says enough. */
	letter: string;
	color: string;
	label: string;
	/** Entries git is not tracking on purpose, drawn back rather than marked. */
	isMuted: boolean;
};

const marksByStatus: Record<string, GitMark> = {
	conflicted: { letter: "!", color: colors.red, label: "Conflicted", isMuted: false },
	modified: { letter: "M", color: colors.amber, label: "Modified", isMuted: false },
	added: { letter: "A", color: colors.green, label: "Added", isMuted: false },
	deleted: { letter: "D", color: colors.red, label: "Deleted", isMuted: false },
	renamed: { letter: "R", color: colors.blue, label: "Renamed", isMuted: false },
	untracked: { letter: "U", color: colors.green, label: "Untracked", isMuted: false },
	ignored: { letter: "", color: colors.faint, label: "Ignored", isMuted: true },
};

export function gitMarkFor(status: string | undefined): GitMark | null {
	if (!status) {
		return null;
	}
	const mark = marksByStatus[status];
	if (!mark) {
		return null;
	}
	return mark;
}
