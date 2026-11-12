<!--
	Yazi's task manager: the file operations under way, walked with j and k.
	x cancels the highlighted one, Enter shows what it is working on and what
	went wrong, w or Escape closes.
-->
<script lang="ts">
	import { Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import {
		isFinished,
		operationDetailLine,
		operationLabel,
		operationStatusLine,
		type OperationProgress,
	} from "../lib/fileops";
	import { colors, radius, space, textSize } from "../theme";
	import Dialog from "./Dialog.svelte";

	let {
		operations,
		onCancel,
		onClose,
	}: {
		operations: OperationProgress[];
		onCancel: (id: string) => void;
		onClose: () => void;
	} = $props();

	let cursor = $state(0);
	let inspectedId = $state("");

	const highlighted = $derived(operations[Math.min(cursor, operations.length - 1)]);

	function handleKey(event: KeyEvent) {
		const step = cursorStep(event.key);
		if (step !== 0) {
			moveCursor(step);
			return;
		}
		if (event.key === "w") {
			onClose();
			return;
		}
		actOnHighlighted(event.key);
	}

	function cursorStep(key: string): number {
		if (key === "j" || key === "ArrowDown") {
			return 1;
		}
		if (key === "k" || key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

	function moveCursor(step: number) {
		if (operations.length === 0) {
			return;
		}
		cursor = Math.max(0, Math.min(operations.length - 1, cursor + step));
	}

	function actOnHighlighted(key: string) {
		if (!highlighted) {
			return;
		}
		if (key === "x" && !isFinished(highlighted)) {
			onCancel(highlighted.id);
			return;
		}
		if (key === "Enter") {
			toggleInspection(highlighted.id);
		}
	}

	function toggleInspection(id: string) {
		if (inspectedId === id) {
			inspectedId = "";
			return;
		}
		inspectedId = id;
	}

	function rowBackground(operation: OperationProgress): string {
		if (operation === highlighted) {
			return colors.hover;
		}
		return "transparent";
	}
</script>

<Dialog title="Tasks" {onClose} onKey={handleKey}>
	{#if operations.length === 0}
		<Text style={{ fontSize: textSize.xs, color: colors.dim }}>No tasks running.</Text>
	{/if}

	{#each operations as operation (operation.id)}
		<View style={{ gap: space[1], padding: space[2], radius: radius.md, background: rowBackground(operation) }}>
			<View style={{ direction: "row", align: "baseline", gap: space[2] }}>
				<Text style={{ shrink: 0, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
					{operationLabel(operation.kind)}
				</Text>
				<Text style={{ flex: 1, minWidth: 0, truncate: true, textAlign: "right", fontSize: textSize.xs, color: colors.dim }}>
					{operationStatusLine(operation)}
				</Text>
			</View>
			{#if inspectedId === operation.id}
				<Text style={{ fontSize: textSize["2xs"], color: colors.faint }}>{operationDetailLine(operation)}</Text>
				{#each operation.failures as failure (failure.path)}
					<Text style={{ fontSize: textSize["2xs"], color: colors.red }}>{failure.path}: {failure.message}</Text>
				{/each}
			{/if}
		</View>
	{/each}

	<Text style={{ fontSize: textSize["2xs"], color: colors.faint }}>j k move · Enter inspect · x cancel · w close</Text>
</Dialog>
