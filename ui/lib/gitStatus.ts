/**
 * How the states git reports are drawn. The Go side folds a folder's contents
 * into the folder itself, so anything here applies to a single entry.
 */
export type GitMark = {
	/** The letter shown beside the entry, empty where the tone says enough. */
	letter: string;
	toneClass: string;
	label: string;
	/** Entries git is not tracking on purpose, drawn back rather than marked. */
	isMuted: boolean;
};

const marksByStatus: Record<string, GitMark> = {
	conflicted: { letter: "!", toneClass: "text-red", label: "Conflicted", isMuted: false },
	modified: { letter: "M", toneClass: "text-amber", label: "Modified", isMuted: false },
	added: { letter: "A", toneClass: "text-green", label: "Added", isMuted: false },
	deleted: { letter: "D", toneClass: "text-red", label: "Deleted", isMuted: false },
	renamed: { letter: "R", toneClass: "text-blue", label: "Renamed", isMuted: false },
	untracked: { letter: "U", toneClass: "text-green", label: "Untracked", isMuted: false },
	ignored: { letter: "", toneClass: "text-faint", label: "Ignored", isMuted: true },
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
