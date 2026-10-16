<!--
	A modal: a dimmed backdrop over the whole window and a card in the middle,
	closed by Escape. Built from the design system's Card and SectionHeader, which
	have no backdrop or focus handling of their own. Place it as a late child of
	the root view so it draws above everything before it.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import Card from "./design/Card.svelte";
	import SectionHeader from "./design/SectionHeader.svelte";
	import { scrim, shadow, space } from "../theme";

	let {
		title,
		onClose,
		children,
		footer,
	}: {
		title: string;
		onClose: () => void;
		children: Snippet;
		footer?: Snippet;
	} = $props();

	let panel = $state<View>();

	$effect(() => {
		panel?.focus();
	});

	function handleKey(event: KeyEvent) {
		if (event.key !== "Escape") {
			return;
		}
		event.stopPropagation();
		onClose();
	}
</script>

<!-- The press handler gives the backdrop a hitbox, so the window behind it takes no presses. -->
<View style={{ ...scrim, padding: space[2] }} onpress={() => undefined}>
	<View bind:this={panel} focusable onkey={handleKey} style={{ width: "full", maxWidth: 448 }}>
		<Card surface="elevated" padding="none" style={{ gap: space[2], padding: space[2], shadow: shadow.lg }}>
			<SectionHeader {title} />
			{@render children()}
			{#if footer}
				<View style={{ direction: "row", wrap: true, justify: "end", gap: space[2] }}>{@render footer()}</View>
			{/if}
		</Card>
	</View>
</View>
