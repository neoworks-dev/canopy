<script lang="ts">
	import { columnSummary } from "./columns";
	import EntryIcon from "./EntryIcon.svelte";
	import { gitMarkFor } from "./gitStatus";
	import RenameField from "./RenameField.svelte";
	import { entryElementId, type EntryViewProps } from "./entryList";

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
	}: EntryViewProps = $props();

	let grid = $state<HTMLDivElement>();

	// Tiles per row come from auto-fill, which decides them from the available
	// width, so both the count and the tile height are read back from the layout.
	$effect(() => {
		if (!grid) {
			return;
		}
		const observer = new ResizeObserver(reportMetrics);
		observer.observe(grid);
		return () => observer.disconnect();
	});

	// Re-measures once tiles exist, which is when there is one to measure.
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
		const tile = grid?.querySelector('[role="option"]');
		if (!(tile instanceof HTMLElement) || !grid) {
			return;
		}
		const gap = parseFloat(getComputedStyle(grid).rowGap);
		onMetrics({ itemHeight: tile.offsetHeight + gap, columns: tileColumns(grid), leadingHeight: 0 });
	}

	function tileColumns(element: HTMLDivElement): number {
		const tracks = getComputedStyle(element).gridTemplateColumns.split(" ");
		return Math.max(1, tracks.length);
	}
</script>

<!--
	The space the unmounted tiles stand in for is padding on the wrapper rather
	than tiles of its own, so it never picks up a grid gap the window maths does
	not account for.
-->
<div style="padding-top: {topPadding}px; padding-bottom: {bottomPadding}px">
	<div
		bind:this={grid}
		class="grid gap-2"
		style="grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr))"
	>
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
				class="flex min-w-0 flex-col items-center gap-2 rounded-lg p-2 transition duration-fast"
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
				<span class="relative flex">
					<EntryIcon {entry} size="xl" />
					{#if markLetter(entry.name)}
						<span
							class="absolute -top-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full bg-raised text-2xs font-medium {markTone(entry.name)}"
							title={markLabel(entry.name)}
						>
							{markLetter(entry.name)}
						</span>
					{/if}
				</span>
				{#if entry.path === renamingPath}
					<RenameField
						initialName={entry.name}
						onCommit={onRenameCommit}
						onCancel={onRenameCancel}
					/>
				{:else}
					<span class="w-full truncate text-center text-xs font-medium text-default">
						{entry.name}
					</span>
				{/if}
				{#if columns.length > 0}
					<span class="w-full truncate text-center text-2xs text-dim">
						{columnSummary(entry, columns)}
					</span>
				{/if}
			</div>
		{/each}
	</div>
</div>
