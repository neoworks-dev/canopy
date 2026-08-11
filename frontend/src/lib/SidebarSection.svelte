<script lang="ts">
	import CaretRightIcon from "phosphor-svelte/lib/CaretRightIcon";
	import type { Snippet } from "svelte";

	// SectionHeader from @neoworks-dev/ui is a page heading: semibold body text
	// with no collapse. A rail group needs a quiet caps label that folds, so it
	// is built here from the same tokens.
	let {
		title,
		isCollapsed = false,
		onToggle,
		children,
	}: {
		title: string;
		isCollapsed?: boolean;
		onToggle: () => void;
		children: Snippet;
	} = $props();
</script>

<section class="flex flex-col">
	<button
		type="button"
		onclick={onToggle}
		aria-expanded={!isCollapsed}
		class="flex items-center gap-1 rounded-md px-2 py-1 text-left text-2xs tracking-caps text-faint uppercase transition duration-fast hover:text-muted"
	>
		<span
			class="flex shrink-0 transition-transform duration-fast"
			class:rotate-90={!isCollapsed}
		>
			<CaretRightIcon size={10} weight="bold" />
		</span>
		<span class="truncate">{title}</span>
	</button>

	{#if !isCollapsed}
		<div class="flex flex-col">
			{@render children()}
		</div>
	{/if}
</section>
