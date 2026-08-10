<script lang="ts">
	import type { filesystem } from "../../wailsjs/go/models";
	import type { ColumnDefinition } from "./columns";
	import EntryIcon from "./EntryIcon.svelte";

	// ListRow from @neoworks-dev/ui carries a title/subtitle pair and no column
	// tracks, so a row with selectable state and configurable columns is built
	// here from the same tokens.
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

	function rowClasses(entry: filesystem.Entry): string {
		if (selectedPaths.has(entry.path)) {
			return "bg-raised ring-1 ring-inset ring-line-strong";
		}
		return "hover:bg-hover";
	}
</script>

<div class="flex min-w-0 flex-col gap-2">
	<div class="flex items-center gap-2 px-2 text-2xs tracking-caps text-faint uppercase">
		<span class="w-8 shrink-0"></span>
		<span class="min-w-0 flex-1">Name</span>
		{#each columns as column (column.id)}
			<span class="shrink-0 {column.widthClass} {column.valueClass}">{column.label}</span>
		{/each}
	</div>

	{#each entries as entry, index (entry.path)}
		<button
			type="button"
			onclick={(event) => onSelect(index, event)}
			ondblclick={() => onOpen(entry)}
			aria-pressed={selectedPaths.has(entry.path)}
			class="flex w-full items-center gap-2 rounded-lg p-2 text-left transition duration-fast select-none {rowClasses(
				entry,
			)}"
		>
			<EntryIcon {entry} />
			<span class="min-w-0 flex-1 truncate text-sm font-medium text-default">{entry.name}</span>
			{#each columns as column (column.id)}
				<span class="shrink-0 truncate text-xs text-dim {column.widthClass} {column.valueClass}">
					{column.valueOf(entry)}
				</span>
			{/each}
		</button>
	{/each}
</div>
