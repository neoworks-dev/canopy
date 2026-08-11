/**
 * Mirrors what the Go scanner reports while it walks a folder. The totals
 * travel over Wails events rather than method calls, so they are not part of the
 * generated bindings and are declared here instead.
 *
 * Nothing is cached between selections: a folder's own timestamp says nothing
 * about what changed deeper down, so a remembered total would go quietly stale.
 */
import { formatFileSize } from "./format";

export type Measurement = {
	id: string;
	path: string;
	bytes: number;
	files: number;
	directories: number;
	/** Entries the walk could not read, which contribute nothing to the total. */
	unreadable: number;
	cancelled: boolean;
};

export const usageEvents = {
	progress: "usage:progress",
	finished: "usage:finished",
};

/** The size row: a running total says so, a finished one stands on its own. */
export function measuredSizeLabel(measurement: Measurement | null, counting: boolean): string {
	if (!measurement) {
		return "Measuring…";
	}
	if (counting) {
		return `${formatFileSize(measurement.bytes)} so far`;
	}
	return formatFileSize(measurement.bytes);
}

/** What the folder holds, in the counts a size alone does not tell you. */
export function measuredContentsLabel(measurement: Measurement | null): string {
	if (!measurement) {
		return "—";
	}
	const parts = [countOf(measurement.files, "file"), countOf(measurement.directories, "folder")];
	if (measurement.unreadable > 0) {
		parts.push(`${formatCount(measurement.unreadable)} unreadable`);
	}
	return parts.join(", ");
}

function countOf(count: number, noun: string): string {
	if (count === 1) {
		return `1 ${noun}`;
	}
	return `${formatCount(count)} ${noun}s`;
}

function formatCount(count: number): string {
	return count.toLocaleString();
}
