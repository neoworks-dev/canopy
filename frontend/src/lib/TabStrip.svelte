<script lang="ts">
	import PlusIcon from "phosphor-svelte/lib/PlusIcon";
	import XIcon from "phosphor-svelte/lib/XIcon";
	import ColumnsIcon from "phosphor-svelte/lib/ColumnsIcon";
	import { tabTitle, type PaneGroup } from "./tabs";

	// @neoworks-dev/ui has no tab bar: NavBar is a fixed bottom rail built around
	// hrefs and a single active route, with no close or reorder. This is the same
	// tokens at the proportions a pane header needs.
	let {
		group,
		isActive,
		canSplit,
		onSelect,
		onClose,
		onOpen,
		onSplit,
	}: {
		group: PaneGroup;
		isActive: boolean;
		canSplit: boolean;
		onSelect: (tabId: string) => void;
		onClose: (tabId: string) => void;
		onOpen: () => void;
		onSplit: () => void;
	} = $props();
</script>

<div class="flex shrink-0 items-center gap-1 overflow-x-auto">
	{#each group.tabs as tab (tab.id)}
		{@const selected = tab.id === group.activeTabId}
		<div
			class="group/tab flex min-w-0 items-center rounded-md transition duration-fast"
			class:bg-raised={selected && isActive}
			class:bg-hover={selected && !isActive}
		>
			<button
				type="button"
				onclick={() => onSelect(tab.id)}
				aria-current={selected ? "page" : undefined}
				title={tabTitle(tab)}
				class="min-w-0 max-w-40 truncate rounded-md py-1 pr-1 pl-2 text-xs transition duration-fast"
				class:text-default={selected}
				class:text-muted={!selected}
				class:hover:text-default={!selected}
			>
				{tabTitle(tab)}
			</button>
			<button
				type="button"
				onclick={() => onClose(tab.id)}
				aria-label="Close {tabTitle(tab)}"
				class="mr-1 shrink-0 rounded-sm p-0.5 text-faint opacity-0 transition duration-fast group-hover/tab:opacity-100 hover:bg-hover hover:text-default focus-visible:opacity-100"
			>
				<XIcon size={10} />
			</button>
		</div>
	{/each}

	<button
		type="button"
		onclick={onOpen}
		aria-label="New tab"
		class="shrink-0 rounded-md p-1 text-faint transition duration-fast hover:bg-hover hover:text-default"
	>
		<PlusIcon size={12} />
	</button>

	<div class="flex-1"></div>

	{#if canSplit}
		<button
			type="button"
			onclick={onSplit}
			aria-label="Split the view"
			class="shrink-0 rounded-md p-1 text-faint transition duration-fast hover:bg-hover hover:text-default"
		>
			<ColumnsIcon size={12} />
		</button>
	{/if}
</div>
