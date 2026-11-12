<!--
	Yazi's layout: the parent folder on the left and the folder on screen beside
	it, four times as wide. The preview of the hovered entry is the pane's aside,
	which takes the remaining three parts.

	Only the folder on screen is the pane's own listing, so it is the one that
	carries the selection, answers to the keyboard and can be dragged from; the
	parent is read separately and is there to step back into.
-->
<script lang="ts">
	import { List, Text, View, type Layout } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import type { ColumnDefinition } from "../lib/columns";
	import { entryRowHeight, entryValueColor, type EntryState } from "../lib/entryStyles";
	import type { EntryViewProps } from "../lib/entryList";
	import { columnChain } from "../lib/millerColumns";
	import type { SortOrder } from "../lib/sorting";
	import { colors, textSize } from "../theme";
	import EntryLine from "./EntryLine.svelte";
	import EntrySlot from "./EntrySlot.svelte";
	import MillerColumn from "./MillerColumn.svelte";

	// Yazi's default ratio is 1:4:3; the 3 is the preview beside this pair.
	const parentShare = 1;
	const currentShare = 4;

	let {
		entries,
		columns,
		selectedPaths,
		cutPaths,
		copiedPaths,
		focusIndex,
		onPress,
		dragPathsFor,
		onDragStart,
		onDragEnd,
		onDropOnFolder,
		canDropInto,
		onLayout,
		onScroll,
		currentPath,
		sortOrder,
		showHidden,
		onDropIntoPath,
		onNavigate,
	}: Pick<
		EntryViewProps,
		| "entries"
		| "columns"
		| "selectedPaths"
		| "cutPaths"
		| "copiedPaths"
		| "focusIndex"
		| "onPress"
		| "dragPathsFor"
		| "onDragStart"
		| "onDragEnd"
		| "onDropOnFolder"
		| "canDropInto"
		| "onLayout"
		| "onScroll"
	> & {
		currentPath: string;
		sortOrder: SortOrder;
		showHidden: boolean;
		/** Something dragged here was dropped into the parent folder or one of its folders. */
		onDropIntoPath: (folder: string, sources: string[]) => void;
		onNavigate: (path: string) => void;
	} = $props();

	let list = $state<ReturnType<typeof List<filesystem.Entry>>>();
	// Where the pane and the current column sit in the window, which says how far
	// in the entries start.
	let paneX = $state(0);
	let currentX = $state(0);
	let currentWidth = $state(0);

	// The chain of two is the parent and the folder on screen; the folder on
	// screen is drawn from the entries the pane already holds.
	const parentPath = $derived(columnChain(currentPath, 2).slice(0, -1)[0]);

	/** Brings an entry into view, moving the list only as far as it takes. */
	export function scrollToIndex(index: number) {
		list?.scrollToIndex(index, { align: "nearest" });
	}

	$effect(() => {
		onLayout({
			columns: 1,
			cellWidth: currentWidth,
			cellHeight: entryRowHeight,
			hitWidth: currentWidth,
			hitHeight: entryRowHeight,
			left: currentX - paneX,
			top: 0,
		});
	});

	function openFromParent(entry: filesystem.Entry) {
		if (!entry.isDir) {
			return;
		}
		onNavigate(entry.path);
	}

	function stateOf(entry: filesystem.Entry, index: number, isDropOver: boolean): EntryState {
		return {
			isSelected: selectedPaths.has(entry.path),
			isFocused: index === focusIndex,
			isDropOver,
			isCut: cutPaths.has(entry.path),
			isCopied: copiedPaths.has(entry.path),
			isMuted: false,
		};
	}

	function linemodeText(entry: filesystem.Entry, shown: ColumnDefinition[]): string {
		return shown.map((column) => column.valueOf(entry)).join("  ");
	}
</script>

<View style={{ flex: 1, minHeight: 0, direction: "row" }} onlayout={(layout: Layout) => (paneX = layout.x)}>
	{#if parentPath}
		<MillerColumn
			path={parentPath}
			{currentPath}
			{sortOrder}
			{showHidden}
			share={parentShare}
			{canDropInto}
			onDropPaths={onDropIntoPath}
			onOpen={openFromParent}
		/>
	{/if}

	<View
		style={{ flex: currentShare, minWidth: 0 }}
		onlayout={(layout: Layout) => (currentX = layout.x)}
		onresize={(layout: Layout) => (currentWidth = layout.width)}
	>
		{#if entries.length === 0}
			<Text style={{ padding: 8, fontSize: textSize.xs, color: colors.faint }}>No items</Text>
		{/if}
		<List
			bind:this={list}
			items={entries}
			rowHeight={entryRowHeight}
			key={(entry) => entry.path}
			style={{ flex: 1, minHeight: 0 }}
			onscroll={(position) => onScroll(position.y)}
		>
			{#snippet row(entry, index)}
				<EntrySlot
					{entry}
					dragPaths={dragPathsFor(entry)}
					onDragStart={() => onDragStart(index)}
					{onDragEnd}
					{onDropOnFolder}
					{canDropInto}
				>
					{#snippet children(isDropOver)}
						{@const rowState = stateOf(entry, index, isDropOver)}
						<EntryLine {entry} state={rowState} onpress={(event) => onPress(index, event)}>
							{#snippet trailing()}
								<Text
									style={{
										shrink: 0,
										fontSize: textSize.xs,
										color: entryValueColor(rowState),
									}}
								>
									{linemodeText(entry, columns)}
								</Text>
							{/snippet}
						</EntryLine>
					{/snippet}
				</EntrySlot>
			{/snippet}
		</List>
	</View>
</View>
