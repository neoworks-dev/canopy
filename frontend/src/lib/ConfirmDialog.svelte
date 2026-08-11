<script lang="ts">
	import { Button } from "@neoworks-dev/ui";
	import Dialog from "./Dialog.svelte";

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
	<p class="text-sm text-muted">{message}</p>

	{#snippet footer()}
		<Button size="sm" variant="ghost" onclick={onCancel}>Cancel</Button>
		<Button size="sm" variant="surface" onclick={onConfirm}>{confirmLabel}</Button>
	{/snippet}
</Dialog>
