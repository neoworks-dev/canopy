<script lang="ts">
	import { Button, Card } from "@neoworks-dev/ui";
	import PauseIcon from "phosphor-svelte/lib/PauseIcon";
	import PlayIcon from "phosphor-svelte/lib/PlayIcon";
	import XIcon from "phosphor-svelte/lib/XIcon";
	import {
		completionPercent,
		isFinished,
		isMeasuring,
		movesData,
		operationLabel,
		type OperationProgress,
	} from "./fileops";
	import { formatFileSize } from "./format";

	// @neoworks-dev/ui has no toast or progress bar: LoadingSpinner is
	// indeterminate and StatusBadge is a static label, so the queue is rendered
	// as stacked Cards on the shared tokens.
	let {
		operations,
		onPause,
		onResume,
		onCancel,
		onDismiss,
	}: {
		operations: OperationProgress[];
		onPause: (id: string) => void;
		onResume: (id: string) => void;
		onCancel: (id: string) => void;
		onDismiss: (id: string) => void;
	} = $props();

	function statusLine(operation: OperationProgress): string {
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

	function detailLine(operation: OperationProgress): string {
		if (isFinished(operation)) {
			return operation.destination;
		}
		return operation.currentPath;
	}
</script>

{#if operations.length > 0}
	<div class="pointer-events-none fixed right-2 bottom-2 z-40 flex w-80 flex-col gap-2">
		{#each operations as operation (operation.id)}
			<Card
				surface="elevated"
				padding="none"
				class="pointer-events-auto flex flex-col gap-2 p-2 shadow-lg"
			>
				<div class="flex items-center gap-2">
					<div class="min-w-0 flex-1">
						<p class="truncate text-sm font-medium text-default">
							{operationLabel(operation.kind)}
						</p>
						<p class="truncate text-2xs text-faint">{detailLine(operation)}</p>
					</div>
					{#if !isFinished(operation)}
						{#if operation.status === "paused"}
							<Button size="sm" variant="ghost" icon={PlayIcon} onclick={() => onResume(operation.id)} />
						{:else}
							<Button size="sm" variant="ghost" icon={PauseIcon} onclick={() => onPause(operation.id)} />
						{/if}
						<Button size="sm" variant="ghost" icon={XIcon} onclick={() => onCancel(operation.id)} />
					{:else}
						<Button size="sm" variant="ghost" icon={XIcon} onclick={() => onDismiss(operation.id)} />
					{/if}
				</div>

				{#if !isFinished(operation)}
					<div class="h-1 overflow-hidden rounded-full bg-raised">
						<div class="h-full bg-line-strong" style="width: {completionPercent(operation)}%"></div>
					</div>
				{/if}

				<p class="text-xs text-dim">{statusLine(operation)}</p>

				{#each operation.failures as failure (failure.path)}
					<p class="truncate text-2xs text-red">{failure.path}: {failure.message}</p>
				{/each}
			</Card>
		{/each}
	</div>
{/if}
