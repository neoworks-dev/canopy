<script lang="ts">
	import type { ColumnDefinition } from "./columns";
	import EntryIcon from "./EntryIcon.svelte";
	import RenameField from "./RenameField.svelte";
	import { entryElementId, entryStateClasses, type EntryViewProps } from "./entryList";

	// ListRow from @neoworks-dev/ui carries a title/subtitle pair and no column
	// tracks, so a row with selectable state and configurable columns is built
	// here from the same tokens.
	let {
		entries,
		columns,
		selectedPaths,
		focusIndex,
		renamingPath,
		onSelect,
		onOpen,
		onContextMenu,
		onRenameCommit,
		onRenameCancel,
	}: EntryViewProps = $props();

	function valueClasses(column: ColumnDefinition): string {
		return `shrink-0 truncate text-xs text-dim ${column.widthClass} ${column.valueClass}`;
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
		<!--
			Keyboard interaction belongs to the listbox container, which owns focus
			and drives these options through aria-activedescendant.
		-->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			id={entryElementId(index)}
			role="option"
			tabindex="-1"
			aria-selected={selectedPaths.has(entry.path)}
			onclick={(event) => onSelect(index, event)}
			ondblclick={() => onOpen(entry)}
			oncontextmenu={(event) => onContextMenu(index, event)}
			class="flex w-full items-center gap-2 rounded-lg p-2 text-left transition duration-fast {entryStateClasses(
				entry,
				index,
				selectedPaths,
				focusIndex,
			)}"
		>
			<EntryIcon {entry} />
			{#if entry.path === renamingPath}
				<RenameField initialName={entry.name} onCommit={onRenameCommit} onCancel={onRenameCancel} />
			{:else}
				<span class="min-w-0 flex-1 truncate text-sm font-medium text-default">{entry.name}</span>
			{/if}
			{#each columns as column (column.id)}
				<span class={valueClasses(column)}>{column.valueOf(entry)}</span>
			{/each}
		</div>
	{/each}
</div>
