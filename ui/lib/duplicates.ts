/**
 * Mirrors what the duplicate search reports. The findings arrive as events
 * rather than as a command's answer, so their types are declared here.
 */
import { formatFileSize } from "./format";

/** One set of files holding the same bytes, in the order the walk found them. */
export type DuplicateGroup = {
	size: number;
	paths: string[];
};

export type DuplicateReport = {
	id: string;
	root: string;
	stage: "walking" | "comparing" | "done";
	scanned: number;
	candidates: number;
	compared: number;
	unreadable: number;
	/** What deleting all but one file of every group would free. */
	recoverableBytes: number;
	groups: DuplicateGroup[];
	cancelled: boolean;
};

export const duplicateEvents = {
	progress: "duplicates:progress",
	finished: "duplicates:finished",
};

/** What the search is busy with, in the terms the pane says it in. */
export function searchProgressLabel(report: DuplicateReport | null): string {
	if (!report) {
		return "Starting…";
	}
	if (report.stage === "walking") {
		return `Looking through ${formatCount(report.scanned)} files`;
	}
	if (report.stage === "comparing") {
		return `Comparing ${formatCount(report.compared)} of ${formatCount(report.candidates)} files`;
	}
	return finishedLabel(report);
}

function finishedLabel(report: DuplicateReport): string {
	if (report.cancelled) {
		return `Stopped after ${formatCount(report.scanned)} files`;
	}
	if (report.groups.length === 0) {
		return `No duplicates among ${formatCount(report.scanned)} files`;
	}
	return `${formatCount(report.groups.length)} sets of copies among ${formatCount(report.scanned)} files`;
}

export function groupLabel(group: DuplicateGroup): string {
	return `${formatCount(group.paths.length)} copies, ${formatFileSize(group.size)} each`;
}

export function recoverableLabel(report: DuplicateReport): string {
	return `${formatFileSize(report.recoverableBytes)} can be freed`;
}

/**
 * What is left of a group once the files already trashed are taken out. A group
 * down to one file is no longer a set of copies and drops out entirely.
 */
export function remainingGroups(
	groups: DuplicateGroup[],
	removed: Set<string>,
): DuplicateGroup[] {
	const remaining = groups.map((group) => ({
		...group,
		paths: group.paths.filter((path) => !removed.has(path)),
	}));
	return remaining.filter((group) => group.paths.length > 1);
}

/** Every copy but the first, which is what trashing the rest of a group takes. */
export function extraPathsOf(group: DuplicateGroup): string[] {
	return group.paths.slice(1);
}

function formatCount(count: number): string {
	return count.toLocaleString();
}
