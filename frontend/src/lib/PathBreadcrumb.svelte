<script lang="ts">
	import CaretRightIcon from "phosphor-svelte/lib/CaretRightIcon";
	import { toBreadcrumbs } from "./format";

	let {
		path,
		onNavigate,
	}: {
		path: string;
		onNavigate: (path: string) => void;
	} = $props();

	const crumbs = $derived(toBreadcrumbs(path));
</script>

<nav class="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto whitespace-nowrap">
	{#each crumbs as crumb, index (crumb.path)}
		{#if index > 0}
			<CaretRightIcon size={12} class="shrink-0 text-faint" />
		{/if}
		<button
			type="button"
			onclick={() => onNavigate(crumb.path)}
			class="shrink-0 rounded-md px-2 py-1 text-sm text-muted transition duration-fast hover:bg-hover hover:text-default"
			class:text-default={index === crumbs.length - 1}
			class:font-medium={index === crumbs.length - 1}
		>
			{crumb.label}
		</button>
	{/each}
</nav>
