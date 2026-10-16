<!--
	The queue of file operations as a stack of cards in the corner of the window,
	each with its progress, pause and cancel. The design system has no toast or
	progress bar, so the bar is built from its tokens.
-->
<script lang="ts">
	import { Text, View } from "@neoworks/svelte-gpui/ui";
	import { pause, play, x } from "../icons";
	import {
		completionPercent,
		isFinished,
		isMeasuring,
		movesData,
		operationLabel,
		type OperationProgress,
	} from "../lib/fileops";
	import { formatFileSize } from "../lib/format";
	import { colors, radius, shadow, space, textSize } from "../theme";
	import Button from "./design/Button.svelte";
	import Card from "./design/Card.svelte";

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
	<View style={{ position: "absolute", right: space[2], bottom: space[2], width: 320, gap: space[2] }}>
		{#each operations as operation (operation.id)}
			<Card surface="elevated" padding="none" style={{ gap: space[2], padding: space[2], shadow: shadow.lg }}>
				<View style={{ direction: "row", align: "center", gap: space[2] }}>
					<View style={{ flex: 1, minWidth: 0 }}>
						<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
							{operationLabel(operation.kind)}
						</Text>
						<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>
							{detailLine(operation)}
						</Text>
					</View>
					{#if isFinished(operation)}
						<Button size="sm" variant="ghost" icon={x} onpress={() => onDismiss(operation.id)} />
					{:else}
						{#if operation.status === "paused"}
							<Button size="sm" variant="ghost" icon={play} onpress={() => onResume(operation.id)} />
						{:else}
							<Button size="sm" variant="ghost" icon={pause} onpress={() => onPause(operation.id)} />
						{/if}
						<Button size="sm" variant="ghost" icon={x} onpress={() => onCancel(operation.id)} />
					{/if}
				</View>

				{#if !isFinished(operation)}
					<View style={{ height: 4, radius: radius.full, background: colors.raised, overflow: "hidden" }}>
						<View style={{ height: 4, width: `${completionPercent(operation)}%`, background: colors.lineStrong }} />
					</View>
				{/if}

				<Text style={{ fontSize: textSize.xs, color: colors.dim }}>{statusLine(operation)}</Text>

				{#each operation.failures as failure (failure.path)}
					<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.red }}>
						{failure.path}: {failure.message}
					</Text>
				{/each}
			</Card>
		{/each}
	</View>
{/if}
