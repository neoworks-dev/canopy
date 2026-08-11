<script lang="ts">
	import { Card, SectionHeader } from "@neoworks-dev/ui";
	import type { Snippet } from "svelte";

	// @neoworks-dev/ui has no modal: Card is a plain surface with no backdrop or
	// focus handling, and Tooltip is hover-only. This wraps the shared Card with
	// the backdrop, the dialog role and Escape handling.
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

	let panel = $state<HTMLDivElement>();

	$effect(() => {
		panel?.focus();
	});

	function handleKeydown(event: KeyboardEvent) {
		if (event.key !== "Escape") {
			return;
		}
		event.stopPropagation();
		onClose();
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-canvas/80 p-2">
	<div
		bind:this={panel}
		role="dialog"
		aria-modal="true"
		aria-label={title}
		tabindex="-1"
		onkeydown={handleKeydown}
		class="w-full max-w-md focus:outline-none"
	>
		<Card surface="elevated" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
			<SectionHeader {title} />
			{@render children()}
			{#if footer}
				<div class="flex flex-wrap justify-end gap-2">{@render footer()}</div>
			{/if}
		</Card>
	</div>
</div>
