<script lang="ts">
	import { Text } from "@neoworks/svelte-gpui/ui";
	import Button from "./design/Button.svelte";
	import Dialog from "./Dialog.svelte";
	import { colors, textSize } from "../theme";

	let {
		title,
		message,
		confirmLabel,
		onConfirm,
		onCancel,
	}: {
		title: string;
		message: string;
		confirmLabel: string;
		onConfirm: () => void;
		onCancel: () => void;
	} = $props();
</script>

<Dialog {title} onClose={onCancel}>
	<Text style={{ fontSize: textSize.sm, color: colors.muted }}>{message}</Text>

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={onCancel}>Cancel</Button>
		<Button size="sm" variant="surface" onpress={onConfirm}>{confirmLabel}</Button>
	{/snippet}
</Dialog>
