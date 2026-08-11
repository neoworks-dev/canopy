<script lang="ts">
	import type { filesystem } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { entryElementId } from "./entryList";
	import type { ListMetrics } from "./virtualList";
	import MillerColumn from "./MillerColumn.svelte";
	import { columnChain } from "./millerColumns";
	import type { SortOrder } from "./sorting";

	/**
	 * The folder on screen and every folder on the way to it, side by side.
	 *
	 * Only the deepest column is the pane's own listing, so it is the one that
	 * carries the selection, answers to the keyboard and is virtualized; the
	 * columns behind it are read separately and are there to be walked back
	 * through.
	 */
	let {
		entries,
		firstIndex,
		topPadding,
		bottomPadding,
		selectedPaths,
		focusIndex,
		onSelect,
		onOpen,
		onContextMenu,
		onDragStart,
		onMetrics,
		onScroller,
		currentPath,
		sortOrder,
		showHidden,
		onNavigate,
	}: {
		/** The mounted window of the current folder, not the whole of it. */
		entries: filesystem.Entry[];
		firstIndex: number;
		topPadding: number;
		bottomPadding: number;
		selectedPaths: Set<string>;
		focusIndex: number;
		onSelect: (index: number, event: MouseEvent) => void;
		onOpen: (entry: filesystem.Entry) => void;
		onContextMenu: (index: number, event: MouseEvent) => void;
		onDragStart: (index: number, event: DragEvent) => void;
		onMetrics: (metrics: ListMetrics) => void;
		/** Hands the pane the element that scrolls its listing. */
		onScroller: (element: HTMLDivElement | undefined) => void;
		currentPath: string;
		sortOrder: SortOrder;
		showHidden: boolean;
		onNavigate: (path: string) => void;
	} = $props();

	// Enough history to walk back through without the folder on screen being
	// pushed off the far edge of a narrow pane.
	const visibleColumnLimit = 4;

	let trailing = $state<HTMLDivElement>();

	// The chain leaves out the folder on screen: that one is rendered from the
	// entries the pane already holds rather than read a second time.
	const ancestors = $derived(columnChain(currentPath, visibleColumnLimit).slice(0, -1));

	// The pane virtualizes and scrolls whichever element holds its own listing,
	// which in this view is the column at the end of the chain.
	$effect(() => {
		onScroller(trailing);
	});

	// Row heights come from the type scale, so they are read back from a mounted
	// row rather than restated as a number here.
	$effect(() => {
		if (!trailing) {
			return;
		}
		const observer = new ResizeObserver(reportMetrics);
		observer.observe(trailing);
		return () => observer.disconnect();
	});

	// Re-measures once entries exist, which is when there is a row to measure.
	$effect(() => {
		entries.length;
		reportMetrics();
	});

	function reportMetrics() {
		const row = trailing?.querySelector('[role="option"]');
		if (!(row instanceof HTMLElement)) {
			return;
		}
		onMetrics({ itemHeight: row.offsetHeight, columns: 1, leadingHeight: 0 });
	}

	function openFromColumn(entry: filesystem.Entry) {
		if (entry.isDir) {
			onNavigate(entry.path);
			return;
		}
		onOpen(entry);
	}
</script>

<div class="flex h-full items-stretch gap-2">
	{#each ancestors as columnPath (columnPath)}
		<MillerColumn
			path={columnPath}
			{currentPath}
			{sortOrder}
			{showHidden}
			onOpen={openFromColumn}
		/>
	{/each}

	<!-- The deepest column is the pane's vertical viewport, so the pane is handed
		 the element to measure and to scroll. -->
	<div bind:this={trailing} class="flex h-full w-64 shrink-0 flex-col overflow-y-auto">
		<div style="height: {topPadding}px"></div>

		{#each entries as entry, position (entry.path)}
			{@const index = firstIndex + position}
			{@const isSelected = selectedPaths.has(entry.path)}
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
				draggable="true"
				aria-selected={isSelected}
				onclick={(event) => onSelect(index, event)}
				ondblclick={() => onOpen(entry)}
				oncontextmenu={(event) => onContextMenu(index, event)}
				ondragstart={(event) => onDragStart(index, event)}
				class="flex w-full shrink-0 items-center gap-2 rounded-md px-2 py-1 text-left transition duration-fast"
				class:bg-raised={isSelected}
				class:hover:bg-hover={!isSelected}
				class:ring-2={index === focusIndex}
				class:ring-inset={index === focusIndex}
				class:ring-line-strong={index === focusIndex}
			>
				<EntryIcon {entry} />
				<span class="min-w-0 flex-1 truncate text-sm text-default">{entry.name}</span>
				{#if entry.isDir}
					<span class="shrink-0 text-2xs text-faint">›</span>
				{/if}
			</div>
		{/each}

		<div style="height: {bottomPadding}px"></div>
	</div>
</div>
