<script lang="ts">
	import EntryIcon from "./EntryIcon.svelte";
	import RenameField from "./RenameField.svelte";
	import { entryElementId, type EntryViewProps } from "./entryList";

	// ListRow from @neoworks-dev/ui carries a title/subtitle pair and no column
	// tracks, so a row with selectable state and configurable columns is built
	// here from the same tokens.
	let {
		entries,
		columns,
		selectedPaths,
		cutPaths,
		focusIndex,
		renamingPath,
		onSelect,
		onOpen,
		onContextMenu,
		onRenameCommit,
		onRenameCancel,
	}: EntryViewProps = $props();
</script>

<div class="flex min-w-0 flex-col">
	<div class="flex items-center gap-2 px-2 pb-1 text-2xs tracking-caps text-faint uppercase">
		<span class="w-8 shrink-0"></span>
		<span class="min-w-0 flex-1">Name</span>
		{#each columns as column (column.id)}
			<span class="shrink-0 {column.widthClass} {column.valueClass}">{column.label}</span>
		{/each}
	</div>

	{#each entries as entry, index (entry.path)}
		{@const isSelected = selectedPaths.has(entry.path)}
		<!--
			Keyboard interaction belongs to the listbox container, which owns focus
			and drives these options through aria-activedescendant.
		-->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			id={entryElementId(index)}
			role="option"
			tabindex="-1"
			aria-selected={isSelected}
			onclick={(event) => onSelect(index, event)}
			ondblclick={() => onOpen(entry)}
			oncontextmenu={(event) => onContextMenu(index, event)}
			class="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left transition duration-fast"
			class:bg-raised={isSelected}
			class:hover:bg-hover={!isSelected}
			class:ring-2={index === focusIndex}
			class:ring-inset={index === focusIndex}
			class:ring-line-strong={index === focusIndex}
			class:opacity-50={cutPaths.has(entry.path)}
		>
			<EntryIcon {entry} />
			{#if entry.path === renamingPath}
				<RenameField initialName={entry.name} onCommit={onRenameCommit} onCancel={onRenameCancel} />
			{:else}
				<span class="min-w-0 flex-1 truncate text-sm text-default">{entry.name}</span>
			{/if}
			{#each columns as column (column.id)}
				<span class="shrink-0 truncate text-xs text-dim {column.widthClass} {column.valueClass}">
					{column.valueOf(entry)}
				</span>
			{/each}
		</div>
	{/each}
</div>
