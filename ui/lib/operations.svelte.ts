import {
	CancelOperation,
	PauseOperation,
	Redo,
	RedoLabel,
	ResolveConflict,
	ResumeOperation,
	StartOperation,
	Undo,
	UndoLabel,
} from "../backend/FileService";
import { EventsOn } from "../backend/events";
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
 * Frontend half of the operation engine: mirrors the queue the backend runs,
 * holds the conflict prompt waiting for an answer, and keeps the undo and redo
 * labels current.
 */
export class OperationQueue {
	operations = $state<OperationProgress[]>([]);
	conflict = $state<ConflictPrompt | null>(null);
	undoLabel = $state("");
	redoLabel = $state("");
	errorMessage = $state("");

	private onFinished: () => void;
	// Set for a paste that was asked to replace whatever is in the way, so no
	// conflict is put to the user.
	private replacesConflicts = false;

	constructor(onFinished: () => void) {
		this.onFinished = onFinished;
	}

	/** Subscribes to the engine's events, returning the unsubscribe callback. */
	listen(): () => void {
		const stops = [
			EventsOn(operationEvents.progress, (progress: OperationProgress) => this.apply(progress)),
			EventsOn(operationEvents.conflict, (prompt: ConflictPrompt) => this.receiveConflict(prompt)),
			EventsOn(operationEvents.finished, (progress: OperationProgress) => this.complete(progress)),
		];
		this.refreshHistoryLabels();
		return () => stops.forEach((stop) => stop());
	}

	/** Reports whether the engine took the operation on. */
	async start(kind: OperationKind, sources: string[], destination: string): Promise<boolean> {
		if (sources.length === 0) {
			return false;
		}
		this.errorMessage = "";
		try {
			await StartOperation(kind, sources, destination);
		} catch (error) {
			this.errorMessage = describeError(error);
			return false;
		}
		return true;
	}

	/** Like `start`, but a destination that already exists is replaced without asking. */
	async startReplacing(kind: OperationKind, sources: string[], destination: string) {
		this.replacesConflicts = true;
		const started = await this.start(kind, sources, destination);
		if (!started) {
			this.replacesConflicts = false;
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
		await this.step(Undo);
	}

	async redo() {
		await this.step(Redo);
	}

	async refreshHistoryLabels() {
		this.undoLabel = await labelOrEmpty(UndoLabel);
		this.redoLabel = await labelOrEmpty(RedoLabel);
	}

	private async step(take: () => Promise<string>) {
		this.errorMessage = "";
		try {
			await take();
		} catch (error) {
			this.errorMessage = describeError(error);
		}
		await this.refreshHistoryLabels();
		this.onFinished();
	}

	private apply(progress: OperationProgress) {
		const index = this.operations.findIndex((operation) => operation.id === progress.id);
		if (index < 0) {
			this.operations = [...this.operations, progress];
			return;
		}
		this.operations = this.operations.map((operation) => replaceMatching(operation, progress));
	}

	private receiveConflict(prompt: ConflictPrompt) {
		if (this.replacesConflicts) {
			ResolveConflict(prompt.operationId, "replace", true);
			return;
		}
		this.conflict = prompt;
	}

	private complete(progress: OperationProgress) {
		this.replacesConflicts = false;
		this.apply(progress);
		this.dropStalePrompt(progress.id);
		this.refreshHistoryLabels();
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

async function labelOrEmpty(read: () => Promise<string>): Promise<string> {
	try {
		return await read();
	} catch {
		return "";
	}
}

function replaceMatching(operation: OperationProgress, progress: OperationProgress): OperationProgress {
	if (operation.id !== progress.id) {
		return operation;
	}
	return progress;
}
