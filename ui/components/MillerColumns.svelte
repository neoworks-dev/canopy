<!--
	The folder on screen and every folder on the way to it, side by side.

	Only the deepest column is the pane's own listing, so it is the one that
	carries the selection, answers to the keyboard and can be dragged from; the
	columns behind it are read separately and are there to be walked back
	through.
-->
<script lang="ts">
	import { List, Scroll, Text, View, merge, type Layout } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import { entryHover, entryLook, entryRowHeight } from "../lib/entryStyles";
	import type { EntryViewProps } from "../lib/entryList";
	import { columnChain } from "../lib/millerColumns";
	import type { SortOrder } from "../lib/sorting";
	import { colors, radius, space, textSize } from "../theme";
	import EntryIcon from "./EntryIcon.svelte";
	import EntrySlot from "./EntrySlot.svelte";
	import MillerColumn from "./MillerColumn.svelte";

	const deepestColumnWidth = 256;
	// A column and the gap after it, which is how far each one pushes the deepest along.
	const columnPitch = 224 + space[2];

	// Enough history to walk back through without the folder on screen being
	// pushed off the far edge of a narrow pane.
	const visibleColumnLimit = 4;

	let {
		entries,
		selectedPaths,
		focusIndex,
		onPress,
		dragPathsFor,
		onDragStart,
		onDragEnd,
		onDropOnFolder,
		onLayout,
		onScroll,
		onOpen,
		currentPath,
		sortOrder,
		showHidden,
		onNavigate,
	}: Pick<
		EntryViewProps,
		| "entries"
		| "selectedPaths"
		| "focusIndex"
		| "onPress"
		| "dragPathsFor"
		| "onDragStart"
		| "onDragEnd"
		| "onDropOnFolder"
		| "onLayout"
		| "onScroll"
	> & {
		onOpen: (entry: filesystem.Entry) => void;
		currentPath: string;
		sortOrder: SortOrder;
		showHidden: boolean;
		onNavigate: (path: string) => void;
	} = $props();

	let list = $state<ReturnType<typeof List<filesystem.Entry>>>();
	let scroller = $state<ReturnType<typeof Scroll>>();
	// The columns need a height to fill, which the scrolling row around them cannot give.
	let height = $state(0);
	// Where the pane and its deepest column sit in the window, which says how far in
	// the entries start once the columns behind have scrolled.
	let paneX = $state(0);
	let paneWidth = $state(0);
	let deepestX = $state(0);

	// The chain leaves out the folder on screen: that one is drawn from the
	// entries the pane already holds rather than read a second time.
	const ancestors = $derived(columnChain(currentPath, visibleColumnLimit).slice(0, -1));
	const chainWidth = $derived(ancestors.length * columnPitch + deepestColumnWidth);

	/** Brings an entry into view, moving the list only as far as it takes. */
	export function scrollToIndex(index: number) {
		list?.scrollToIndex(index, { align: "nearest" });
	}

	// The folder on screen is the deepest column, so the chain is scrolled to show it.
	$effect(() => {
		scrollToDeepest(chainWidth, paneWidth);
	});

	$effect(() => {
		onLayout({
			columns: 1,
			cellWidth: deepestColumnWidth,
			cellHeight: entryRowHeight,
			hitWidth: deepestColumnWidth,
			hitHeight: entryRowHeight,
			left: deepestX - paneX,
			top: 0,
		});
	});

	// The scroll only reaches as far as the chain has been laid out, which is
	// after the frame that changed it.
	function scrollToDeepest(widthOfChain: number, widthOfPane: number) {
		requestAnimationFrame(() => scroller?.scrollTo(Math.max(0, widthOfChain - widthOfPane)));
	}

	function openFromColumn(entry: filesystem.Entry) {
		if (entry.isDir) {
			onNavigate(entry.path);
			return;
		}
		onOpen(entry);
	}

	function stateOf(entry: filesystem.Entry, index: number, isDropOver: boolean) {
		return {
			isSelected: selectedPaths.has(entry.path),
			isFocused: index === focusIndex,
			isDropOver,
			isCut: false,
			isMuted: false,
		};
	}
</script>

<View
	style={{ flex: 1, minHeight: 0 }}
	onresize={(layout: Layout) => {
		height = layout.height;
		paneWidth = layout.width;
	}}
	onlayout={(layout: Layout) => (paneX = layout.x)}
>
	<Scroll bind:this={scroller} horizontal style={{ flex: 1, minHeight: 0 }}>
		<View style={{ direction: "row", gap: space[2], height, width: chainWidth }}>
			{#each ancestors as columnPath (columnPath)}
				<MillerColumn path={columnPath} {currentPath} {sortOrder} {showHidden} onOpen={openFromColumn} />
			{/each}

			<View
				style={{ width: deepestColumnWidth, shrink: 0 }}
				onlayout={(layout: Layout) => (deepestX = layout.x)}
			>
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
						>
							{#snippet children(isDropOver)}
								{@const entryState = stateOf(entry, index, isDropOver)}
								<View
									style={merge(
										{
											flex: 1,
											direction: "row",
											align: "center",
											gap: space[2],
											paddingX: space[1.5],
											radius: radius.md,
										},
										entryLook(entryState),
									)}
									hover={entryHover(entryState)}
									onpress={(event) => onPress(index, event)}
								>
									<EntryIcon {entry} />
									<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.sm, color: colors.text }}>
										{entry.name}
									</Text>
									{#if entry.isDir}
										<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>›</Text>
									{/if}
								</View>
							{/snippet}
						</EntrySlot>
					{/snippet}
				</List>
			</View>
		</View>
	</Scroll>
</View>
