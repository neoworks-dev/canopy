<!--
	A buffer of names, one per line, for bulk create and bulk rename: what Yazi
	hands to $EDITOR, edited in place. Enter breaks the line; Ctrl+Enter applies.
-->
<script lang="ts">
	import { onMount, tick, untrack } from "svelte";
	import { Input, Text, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { colors, radius, space, textSize } from "../theme";
	import Dialog from "./Dialog.svelte";
	import Button from "./design/Button.svelte";

	const visibleRows = 10;

	let {
		title,
		hint,
		initialText = "",
		errorMessage = "",
		submitLabel,
		onSubmit,
		onClose,
	}: {
		title: string;
		hint: string;
		initialText?: string;
		errorMessage?: string;
		submitLabel: string;
		onSubmit: (text: string) => void;
		onClose: () => void;
	} = $props();

	// Seeded once: the buffer is the user's from then on.
	let text = $state(untrack(() => initialText));
	let field = $state<Input>();

	onMount(async () => {
		await tick();
		field?.focus();
	});

	function handleKey(event: KeyEvent) {
		if (event.key !== "Enter" || !event.ctrl) {
			return;
		}
		event.preventDefault();
		onSubmit(text);
	}
</script>

<Dialog {title} {onClose}>
	<Text style={{ fontSize: textSize.xs, color: colors.dim }}>{hint}</Text>

	<Input
		bind:this={field}
		bind:value={text}
		multiline
		rows={visibleRows}
		onkey={handleKey}
		style={{
			padding: space[2],
			radius: radius.lg,
			background: colors.raised,
			fontFamily: "mono",
			fontSize: textSize.xs,
			color: colors.text,
		}}
	/>

	{#if errorMessage}
		<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={onClose}>Cancel</Button>
		<Button size="sm" variant="surface" onpress={() => onSubmit(text)}>{submitLabel}</Button>
	{/snippet}
</Dialog>
