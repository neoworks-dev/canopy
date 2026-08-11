<script lang="ts">
	import EntryIcon from "./EntryIcon.svelte";
	import { gitMarkFor } from "./gitStatus";
	import RenameField from "./RenameField.svelte";
	import SortHeader from "./SortHeader.svelte";
	import { entryElementId, type EntryViewProps } from "./entryList";
	import type { SortKey, SortOrder } from "./sorting";

	// ListRow from @neoworks-dev/ui carries a title/subtitle pair and no column
	// tracks, so a row with selectable state and configurable columns is built
	// here from the same tokens.
	let {
		entries,
		firstIndex,
		topPadding,
		bottomPadding,
		columns,
		selectedPaths,
		cutPaths,
		gitStatuses,
		focusIndex,
		renamingPath,
		dropTargetPath,
		onSelect,
		onOpen,
		onContextMenu,
		onDragStart,
		onRenameCommit,
		onRenameCancel,
		onMetrics,
		order,
		onSort,
	}: EntryViewProps & { order: SortOrder; onSort: (key: SortKey) => void } = $props();

	let list = $state<HTMLDivElement>();
	let header = $state<HTMLDivElement>();

	// Row heights come from the type scale, so they are read back from a mounted
	// row rather than restated as a number here.
	$effect(() => {
		if (!list) {
			return;
		}
		const observer = new ResizeObserver(reportMetrics);
		observer.observe(list);
		return () => observer.disconnect();
	});

	// Re-measures once entries exist, which is when there is a row to measure.
	$effect(() => {
		entries.length;
		reportMetrics();
	});


	function markTone(name: string): string {
		const mark = gitMarkFor(gitStatuses[name]);
		if (!mark) {
			return "";
		}
		return mark.toneClass;
	}

	function markLetter(name: string): string {
		const mark = gitMarkFor(gitStatuses[name]);
		if (!mark) {
			return "";
		}
		return mark.letter;
	}

	function markLabel(name: string): string {
		const mark = gitMarkFor(gitStatuses[name]);
		if (!mark) {
			return "";
		}
		return mark.label;
	}

	// An ignored entry is drawn back rather than marked, the way an editor greys
	// out what the working tree does not follow.
	function isMuted(name: string): boolean {
		const mark = gitMarkFor(gitStatuses[name]);
		if (!mark) {
			return false;
		}
		return mark.isMuted;
	}

	function reportMetrics() {
		const row = list?.querySelector('[role="option"]');
		if (!(row instanceof HTMLElement) || !header) {
			return;
		}
		onMetrics({ itemHeight: row.offsetHeight, columns: 1, leadingHeight: header.offsetHeight });
	}
</script>

<div bind:this={list} class="flex min-w-0 flex-col">
	<div
		bind:this={header}
		class="flex items-center gap-2 px-2 pb-1 text-2xs tracking-caps text-faint uppercase"
	>
		<span class="w-8 shrink-0"></span>
		<span class="w-3 shrink-0"></span>
		<SortHeader
			label="Name"
			sortKey="name"
			widthClass="min-w-0 flex-1"
			valueClass=""
			{order}
			{onSort}
		/>
		{#each columns as column (column.id)}
			<SortHeader
				label={column.label}
				sortKey={column.sortKey}
				widthClass={column.widthClass}
				valueClass={column.valueClass}
				{order}
				{onSort}
			/>
		{/each}
	</div>

	<div style="height: {topPadding}px"></div>

	{#each entries as entry, position (entry.path)}
		{@const index = firstIndex + position}
		{@const isSelected = selectedPaths.has(entry.path)}
		{@const isDropTarget = entry.path === dropTargetPath}
		<!--
			Keyboard interaction belongs to the listbox container, which owns focus
			and drives these options through aria-activedescendant.
		-->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			id={entryElementId(index)}
			data-index={index}
			role="option"
			tabindex="-1"
			draggable={entry.path !== renamingPath}
			aria-selected={isSelected}
			onclick={(event) => onSelect(index, event)}
			ondblclick={() => onOpen(entry)}
			oncontextmenu={(event) => onContextMenu(index, event)}
			ondragstart={(event) => onDragStart(index, event)}
			class="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left transition duration-fast"
			class:bg-raised={isSelected}
			class:hover:bg-hover={!isSelected}
			class:ring-2={index === focusIndex || isDropTarget}
			class:ring-inset={index === focusIndex || isDropTarget}
			class:ring-line-strong={index === focusIndex && !isDropTarget}
			class:ring-action={isDropTarget}
			class:bg-hover={isDropTarget}
			class:opacity-50={cutPaths.has(entry.path)}
			class:opacity-60={isMuted(entry.name)}
		>
			<EntryIcon {entry} />
			<span
				class="w-3 shrink-0 text-center text-2xs font-medium {markTone(entry.name)}"
				title={markLabel(entry.name)}
			>
				{markLetter(entry.name)}
			</span>
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

	<div style="height: {bottomPadding}px"></div>
</div>
