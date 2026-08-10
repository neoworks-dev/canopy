<script lang="ts">
	import type { filesystem } from "../../wailsjs/go/models";
	import { columnSummary, type ColumnDefinition } from "./columns";
	import EntryIcon from "./EntryIcon.svelte";

	let {
		entries,
		columns,
		selectedPaths,
		onSelect,
		onOpen,
	}: {
		entries: filesystem.Entry[];
		columns: ColumnDefinition[];
		selectedPaths: Set<string>;
		onSelect: (index: number, event: MouseEvent) => void;
		onOpen: (entry: filesystem.Entry) => void;
	} = $props();

	function tileClasses(entry: filesystem.Entry): string {
		if (selectedPaths.has(entry.path)) {
			return "bg-raised ring-1 ring-inset ring-line-strong";
		}
		return "hover:bg-hover";
	}
</script>

<div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr))">
	{#each entries as entry, index (entry.path)}
		<button
			type="button"
			onclick={(event) => onSelect(index, event)}
			ondblclick={() => onOpen(entry)}
			aria-pressed={selectedPaths.has(entry.path)}
			class="flex min-w-0 flex-col items-center gap-2 rounded-lg p-2 transition duration-fast select-none {tileClasses(
				entry,
			)}"
		>
			<EntryIcon {entry} size="lg" />
			<span class="w-full truncate text-center text-xs font-medium text-default">{entry.name}</span>
			{#if columns.length > 0}
				<span class="w-full truncate text-center text-2xs text-dim">
					{columnSummary(entry, columns)}
				</span>
			{/if}
		</button>
	{/each}
</div>
