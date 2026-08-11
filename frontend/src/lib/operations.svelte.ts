import {
	CancelOperation,
	PauseOperation,
	ResolveConflict,
	ResumeOperation,
	StartOperation,
	Undo,
	UndoLabel,
} from "../../wailsjs/go/main/FileService";
import { EventsOn } from "../../wailsjs/runtime/runtime";
import { describeError } from "./errors";
import {
	operationEvents,
	type ConflictPrompt,
	type ConflictResolution,
	type OperationKind,
	type OperationProgress,
} from "./fileops";

/** How long a finished operation stays on screen when nothing went wrong. */
const dismissAfterMilliseconds = 4000;

/**
 * Frontend half of the operation engine: mirrors the queue the Go side runs,
 * holds the conflict prompt waiting for an answer, and keeps the undo label
 * current.
 */
export class OperationQueue {
	operations = $state<OperationProgress[]>([]);
	conflict = $state<ConflictPrompt | null>(null);
	undoLabel = $state("");
	errorMessage = $state("");

	private onFinished: () => void;

	constructor(onFinished: () => void) {
		this.onFinished = onFinished;
	}

	/** Subscribes to the engine's events, returning the unsubscribe callback. */
	listen(): () => void {
		const stops = [
			EventsOn(operationEvents.progress, (progress: OperationProgress) => this.apply(progress)),
			EventsOn(operationEvents.conflict, (prompt: ConflictPrompt) => (this.conflict = prompt)),
			EventsOn(operationEvents.finished, (progress: OperationProgress) => this.complete(progress)),
		];
		this.refreshUndoLabel();
		return () => stops.forEach((stop) => stop());
	}

	async start(kind: OperationKind, sources: string[], destination: string) {
		if (sources.length === 0) {
			return;
		}
		this.errorMessage = "";
		try {
			await StartOperation(kind, sources, destination);
		} catch (error) {
			this.errorMessage = describeError(error);
		}
	}

	resolve(resolution: ConflictResolution, applyToAll: boolean) {
		const prompt = this.conflict;
		if (!prompt) {
			return;
		}
		this.conflict = null;
		ResolveConflict(prompt.operationId, resolution, applyToAll);
	}

	pause(operationID: string) {
		PauseOperation(operationID);
	}

	resume(operationID: string) {
		ResumeOperation(operationID);
	}

	cancel(operationID: string) {
		CancelOperation(operationID);
	}

	dismiss(operationID: string) {
		this.operations = this.operations.filter((operation) => operation.id !== operationID);
	}

	async undo() {
		this.errorMessage = "";
		try {
			await Undo();
		} catch (error) {
			this.errorMessage = describeError(error);
		}
		await this.refreshUndoLabel();
		this.onFinished();
	}

	async refreshUndoLabel() {
		try {
			this.undoLabel = await UndoLabel();
		} catch {
			this.undoLabel = "";
		}
	}

	private apply(progress: OperationProgress) {
		const index = this.operations.findIndex((operation) => operation.id === progress.id);
		if (index < 0) {
			this.operations = [...this.operations, progress];
			return;
		}
		this.operations = this.operations.map((operation) => replaceMatching(operation, progress));
	}

	private complete(progress: OperationProgress) {
		this.apply(progress);
		this.dropStalePrompt(progress.id);
		this.refreshUndoLabel();
		this.onFinished();
		this.scheduleDismissal(progress);
	}

	// Cancelling from the progress card ends an operation that may still be
	// showing a conflict dialog nobody can usefully answer any more.
	private dropStalePrompt(operationID: string) {
		if (this.conflict?.operationId !== operationID) {
			return;
		}
		this.conflict = null;
	}

	// A clean run disappears on its own; one with failures stays until the user
	// has read it.
	private scheduleDismissal(progress: OperationProgress) {
		if (progress.failures.length > 0) {
			return;
		}
		setTimeout(() => this.dismiss(progress.id), dismissAfterMilliseconds);
	}
}

function replaceMatching(operation: OperationProgress, progress: OperationProgress): OperationProgress {
	if (operation.id !== progress.id) {
		return operation;
	}
	return progress;
}
