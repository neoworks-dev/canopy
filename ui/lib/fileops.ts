/**
 * Mirrors the payloads the operation engine emits. They arrive as events rather
 * than as a command's answer, so their types are declared here.
 */
import { formatFileSize } from "./format";

export type OperationKind = "copy" | "move" | "trash" | "delete" | "extract" | "compress";

export type OperationStatus = "queued" | "running" | "paused" | "done" | "cancelled";

export type ConflictResolution = "replace" | "skip" | "keepBoth" | "cancel";

export type OperationFailure = {
	path: string;
	message: string;
};

export type OperationProgress = {
	id: string;
	kind: OperationKind;
	status: OperationStatus;
	destination: string;
	totalItems: number;
	doneItems: number;
	/** Zero for kinds that move no data, where only the item counter is honest. */
	totalBytes: number;
	doneBytes: number;
	currentPath: string;
	failures: OperationFailure[];
};

export type ConflictPrompt = {
	operationId: string;
	sourcePath: string;
	destinationPath: string;
	sourceIsDir: boolean;
};

export const operationEvents = {
	progress: "fileops:progress",
	conflict: "fileops:conflict",
	finished: "fileops:finished",
};

const kindLabels: Record<OperationKind, string> = {
	copy: "Copying",
	move: "Moving",
	trash: "Moving to Trash",
	delete: "Deleting",
	extract: "Extracting",
	compress: "Packing",
};

export function operationLabel(kind: OperationKind): string {
	return kindLabels[kind];
}

export function isFinished(progress: OperationProgress): boolean {
	return progress.status === "done" || progress.status === "cancelled";
}

/**
 * Completion as a percentage. Bytes are the truthful measure while data is
 * moving; an operation that moves none falls back to counting items.
 */
export function completionPercent(progress: OperationProgress): number {
	if (progress.totalBytes > 0) {
		return percentOf(progress.doneBytes, progress.totalBytes);
	}
	return percentOf(progress.doneItems, progress.totalItems);
}

/** True while the sizing pass has not produced a total yet. */
export function isMeasuring(progress: OperationProgress): boolean {
	return movesData(progress) && progress.totalBytes === 0;
}

export function movesData(progress: OperationProgress): boolean {
	return progress.kind === "copy" || progress.kind === "move";
}

/** What an operation is doing, in a few words: its counts, or how it ended. */
export function operationStatusLine(operation: OperationProgress): string {
	if (operation.status === "paused") {
		return "Paused";
	}
	if (isFinished(operation)) {
		return finishedLine(operation);
	}
	if (isMeasuring(operation)) {
		return "Sizing up…";
	}
	return `${itemLine(operation)}${byteLine(operation)}`;
}

/** The file being worked on, or where the result went once it is done. */
export function operationDetailLine(operation: OperationProgress): string {
	if (isFinished(operation)) {
		return operation.destination;
	}
	return operation.currentPath;
}

function itemLine(operation: OperationProgress): string {
	return `${operation.doneItems} of ${operation.totalItems}`;
}

function byteLine(operation: OperationProgress): string {
	if (!movesData(operation)) {
		return "";
	}
	return ` · ${formatFileSize(operation.doneBytes)} of ${formatFileSize(operation.totalBytes)}`;
}

function finishedLine(operation: OperationProgress): string {
	if (operation.status === "cancelled") {
		return "Cancelled";
	}
	if (operation.failures.length > 0) {
		return `Finished with ${operation.failures.length} problem(s)`;
	}
	return "Finished";
}

function percentOf(done: number, total: number): number {
	if (total === 0) {
		return 0;
	}
	return Math.min(100, Math.round((done / total) * 100));
}
