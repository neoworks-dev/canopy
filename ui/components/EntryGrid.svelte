<!--
	The listing as tiles: an icon or thumbnail over the name, as many to a row as
	fit. The list underneath holds rows of tiles rather than tiles, which is what
	lets it build only the rows near the window.
-->
<script lang="ts">
	import { List, Text, View, merge, type Layout } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import { columnSummary } from "../lib/columns";
	import { entryHover, entryLook } from "../lib/entryStyles";
	import type { EntryViewProps } from "../lib/entryList";
	import { gitMarkFor } from "../lib/gitStatus";
	import { colors, radius, space, textSize } from "../theme";
	import EntryIcon from "./EntryIcon.svelte";
	import EntrySlot from "./EntrySlot.svelte";
	import RenameField from "./RenameField.svelte";

	/** The least width a tile takes, so the row holds as many as fit. */
	const minimumTileWidth = 136;
	const gap = space[2];
	// The icon, the name under it, and the padding and ring around them.
	const tileHeightWithoutSummary = 136;
	const tileHeightWithSummary = 158;

	type TileRow = { entries: filesystem.Entry[]; firstIndex: number };

	let {
		entries,
		columns,
		selectedPaths,
		cutPaths,
		gitStatuses,
		focusIndex,
		renamingPath,
		onPress,
		dragPathsFor,
		onDragStart,
		onDragEnd,
		onDropOnFolder,
		onRenameCommit,
		onRenameCancel,
		onLayout,
		onScroll,
	}: EntryViewProps = $props();

	let list = $state<ReturnType<typeof List<TileRow>>>();
	let width = $state(0);

	const tilesPerRow = $derived(Math.max(1, Math.floor((width + gap) / (minimumTileWidth + gap))));
	const tileWidth = $derived((width - (tilesPerRow - 1) * gap) / tilesPerRow);
	const tileHeight = $derived(heightOfTile(columns.length));
	const rows = $derived(chunked(entries, tilesPerRow));

	/** Brings an entry into view, moving the list only as far as it takes. */
	export function scrollToIndex(index: number) {
		list?.scrollToIndex(Math.floor(index / tilesPerRow), { align: "nearest" });
	}

	function heightOfTile(columnCount: number): number {
		if (columnCount > 0) {
			return tileHeightWithSummary;
		}
		return tileHeightWithoutSummary;
	}

	function chunked(all: filesystem.Entry[], size: number): TileRow[] {
		const found: TileRow[] = [];
		for (let first = 0; first < all.length; first += size) {
			found.push({ entries: all.slice(first, first + size), firstIndex: first });
		}
		return found;
	}

	$effect(() => {
		onLayout({
			columns: tilesPerRow,
			cellWidth: tileWidth + gap,
			cellHeight: tileHeight + gap,
			hitWidth: tileWidth,
			hitHeight: tileHeight,
			left: 0,
			top: 0,
		});
	});

	function markOf(entry: filesystem.Entry) {
		return gitMarkFor(gitStatuses[entry.name]);
	}

	function markLetter(entry: filesystem.Entry): string {
		const mark = markOf(entry);
		if (!mark) {
			return "";
		}
		return mark.letter;
	}

	function markColor(entry: filesystem.Entry): string {
		const mark = markOf(entry);
		if (!mark) {
			return colors.faint;
		}
		return mark.color;
	}

	function isMuted(entry: filesystem.Entry): boolean {
		const mark = markOf(entry);
		if (!mark) {
			return false;
		}
		return mark.isMuted;
	}

	function stateOf(entry: filesystem.Entry, index: number, isDropOver: boolean) {
		return {
			isSelected: selectedPaths.has(entry.path),
			isFocused: index === focusIndex,
			isDropOver,
			isCut: cutPaths.has(entry.path),
			isMuted: isMuted(entry),
		};
	}

	function emptySlots(row: TileRow): number[] {
		return Array.from({ length: tilesPerRow - row.entries.length }, (_, slot) => slot);
	}
</script>

<View style={{ flex: 1, minHeight: 0 }} onresize={(layout: Layout) => (width = layout.width)}>
	<List
		bind:this={list}
		items={rows}
		rowHeight={tileHeight + gap}
		key={(row) => row.entries[0].path}
		style={{ flex: 1, minHeight: 0 }}
		onscroll={(position) => onScroll(position.y)}
	>
		{#snippet row(tileRow)}
			<View style={{ direction: "row", gap, height: tileHeight }}>
				{#each tileRow.entries as entry, position (entry.path)}
					{@const index = tileRow.firstIndex + position}
					<EntrySlot
						{entry}
						dragPaths={dragPathsFor(entry)}
						isDragDisabled={entry.path === renamingPath}
						onDragStart={() => onDragStart(index)}
						{onDragEnd}
						{onDropOnFolder}
					>
						{#snippet children(isDropOver)}
							{@const tileState = stateOf(entry, index, isDropOver)}
							<View
								style={merge(
									{
										flex: 1,
										gap: space[2],
										padding: space[1.5],
										radius: radius.lg,
									},
									entryLook(tileState),
								)}
								hover={entryHover(tileState)}
								onpress={(event) => onPress(index, event)}
							>
								<View style={{ alignSelf: "center" }}>
									<EntryIcon {entry} size="xl" />
									{#if markLetter(entry)}
										<View
											style={{
												position: "absolute",
												top: -space[1],
												right: -space[1],
												size: 16,
												align: "center",
												justify: "center",
												radius: radius.full,
												background: colors.raised,
											}}
										>
											<Text style={{ fontSize: textSize["2xs"], fontWeight: "medium", color: markColor(entry) }}>
												{markLetter(entry)}
											</Text>
										</View>
									{/if}
								</View>
								{#if entry.path === renamingPath}
									<RenameField
										initialName={entry.name}
										onCommit={onRenameCommit}
										onCancel={onRenameCancel}
										style={{ flex: 0 }}
									/>
								{:else}
									<Text
										style={{
											truncate: true,
											textAlign: "center",
											fontSize: textSize.xs,
											fontWeight: "medium",
											color: colors.text,
										}}
									>
										{entry.name}
									</Text>
								{/if}
								{#if columns.length > 0}
									<Text
										style={{
											truncate: true,
											textAlign: "center",
											fontSize: textSize["2xs"],
											color: colors.dim,
										}}
									>
										{columnSummary(entry, columns)}
									</Text>
								{/if}
							</View>
						{/snippet}
					</EntrySlot>
				{/each}
				{#each emptySlots(tileRow) as slot (slot)}
					<View style={{ flex: 1 }} />
				{/each}
			</View>
		{/snippet}
	</List>
</View>
