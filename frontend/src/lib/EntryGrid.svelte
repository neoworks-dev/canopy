<script lang="ts">
	import { columnSummary } from "./columns";
	import EntryIcon from "./EntryIcon.svelte";
	import RenameField from "./RenameField.svelte";
	import { entryElementId, type EntryViewProps } from "./entryList";

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
		onColumnCountChange,
	}: EntryViewProps & { onColumnCountChange: (count: number) => void } = $props();

	let grid = $state<HTMLDivElement>();

	// Arrow keys step by the number of tiles actually on a row, which auto-fill
	// decides from the available width, so it is read back from the layout.
	$effect(() => {
		if (!grid) {
			return;
		}
		const observer = new ResizeObserver(reportColumnCount);
		observer.observe(grid);
		reportColumnCount();
		return () => observer.disconnect();
	});

	function reportColumnCount() {
		if (!grid) {
			return;
		}
		const tracks = getComputedStyle(grid).gridTemplateColumns.split(" ");
		onColumnCountChange(Math.max(1, tracks.length));
	}
</script>

<div
	bind:this={grid}
	class="grid gap-2"
	style="grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr))"
>
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
			class="flex min-w-0 flex-col items-center gap-2 rounded-lg p-2 transition duration-fast"
			class:bg-raised={isSelected}
			class:hover:bg-hover={!isSelected}
			class:ring-2={index === focusIndex}
			class:ring-inset={index === focusIndex}
			class:ring-line-strong={index === focusIndex}
			class:opacity-50={cutPaths.has(entry.path)}
		>
			<EntryIcon {entry} size="lg" />
			{#if entry.path === renamingPath}
				<RenameField initialName={entry.name} onCommit={onRenameCommit} onCancel={onRenameCancel} />
			{:else}
				<span class="w-full truncate text-center text-xs font-medium text-default">{entry.name}</span>
			{/if}
			{#if columns.length > 0}
				<span class="w-full truncate text-center text-2xs text-dim">
					{columnSummary(entry, columns)}
				</span>
			{/if}
		</div>
	{/each}
</div>
