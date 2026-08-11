/**
 * Mirrors the payloads the Go operation engine emits. These types travel over
 * Wails events rather than method calls, so they are not part of the generated
 * bindings and are declared here instead.
 */
export type OperationKind = "copy" | "move" | "trash" | "delete";

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
};

export function operationLabel(kind: OperationKind): string {
	return kindLabels[kind];
}

export function isFinished(progress: OperationProgress): boolean {
	return progress.status === "done" || progress.status === "cancelled";
}

/** Completion as a percentage, for the progress bar. */
export function completionPercent(progress: OperationProgress): number {
	if (progress.totalItems === 0) {
		return 0;
	}
	return Math.round((progress.doneItems / progress.totalItems) * 100);
}
