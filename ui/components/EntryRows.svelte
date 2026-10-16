<!--
	The listing as rows under a header of sortable columns. Only the rows near the
	window are built, so a folder of a hundred thousand entries costs what a
	screenful does. The design system's ListRow carries a title and a subtitle
	and no column tracks, so a row with selectable state and configurable columns
	is built here from the same tokens.
-->
<script lang="ts">
	import { List, Text, View, merge, type Layout } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import { entryHover, entryLook, entryRowHeight } from "../lib/entryStyles";
	import type { EntryViewProps } from "../lib/entryList";
	import { gitMarkFor } from "../lib/gitStatus";
	import type { SortKey, SortOrder } from "../lib/sorting";
	import { colors, radius, space, textSize } from "../theme";
	import EntryIcon from "./EntryIcon.svelte";
	import EntrySlot from "./EntrySlot.svelte";
	import RenameField from "./RenameField.svelte";
	import SortHeader from "./SortHeader.svelte";

	const headerHeight = 24;
	// The icon, and the mark beside it, are what the header leaves room for.
	const iconColumnWidth = 32;
	const markColumnWidth = 12;
	// A row's own padding and its ring, so the header lines up with what is in the rows.
	const rowInset = space[2];

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
		order,
		onSort,
	}: EntryViewProps & { order: SortOrder; onSort: (key: SortKey) => void } = $props();

	let list = $state<ReturnType<typeof List<filesystem.Entry>>>();
	let width = $state(0);

	/** Brings an entry into view, moving the list only as far as it takes. */
	export function scrollToIndex(index: number) {
		list?.scrollToIndex(index, { align: "nearest" });
	}

	$effect(() => {
		onLayout({
			columns: 1,
			cellWidth: width,
			cellHeight: entryRowHeight,
			hitWidth: width,
			hitHeight: entryRowHeight,
			left: 0,
			top: headerHeight,
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

	// An ignored entry is drawn back rather than marked, the way an editor greys
	// out what the working tree does not follow.
	function isMuted(entry: filesystem.Entry): boolean {
		const mark = markOf(entry);
		if (!mark) {
			return false;
		}
		return mark.isMuted;
	}

	function cellLook(width: number, isRightAligned: boolean, isMonospace: boolean) {
		return merge(
			{ shrink: 0, width, truncate: true, fontSize: textSize.xs, color: colors.dim },
			isRightAligned && { textAlign: "right" },
			isMonospace && { fontFamily: "mono" },
		);
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
</script>

<View style={{ flex: 1, minHeight: 0 }} onresize={(layout: Layout) => (width = layout.width)}>
	<View
		style={{
			direction: "row",
			shrink: 0,
			align: "center",
			gap: space[2],
			height: headerHeight,
			paddingX: rowInset,
			paddingBottom: space[1],
			fontSize: textSize["2xs"],
			uppercase: true,
			color: colors.faint,
		}}
	>
		<View style={{ width: iconColumnWidth, shrink: 0 }} />
		<View style={{ width: markColumnWidth, shrink: 0 }} />
		<SortHeader label="Name" sortKey="name" width={0} isRightAligned={false} {order} {onSort} />
		{#each columns as column (column.id)}
			<SortHeader
				label={column.label}
				sortKey={column.sortKey}
				width={column.width}
				isRightAligned={column.isRightAligned}
				{order}
				{onSort}
			/>
		{/each}
	</View>

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
				isDragDisabled={entry.path === renamingPath}
				onDragStart={() => onDragStart(index)}
				{onDragEnd}
				{onDropOnFolder}
			>
				{#snippet children(isDropOver)}
					{@const rowState = stateOf(entry, index, isDropOver)}
					<View
						style={merge(
							{
								flex: 1,
								direction: "row",
								align: "center",
								gap: space[2],
								paddingX: rowInset - 2,
								radius: radius.md,
							},
							entryLook(rowState),
						)}
						hover={entryHover(rowState)}
						onpress={(event) => onPress(index, event)}
					>
						<EntryIcon {entry} />
						<Text
							style={{
								width: markColumnWidth,
								shrink: 0,
								textAlign: "center",
								fontSize: textSize["2xs"],
								fontWeight: "medium",
								color: markColor(entry),
							}}
						>
							{markLetter(entry)}
						</Text>
						{#if entry.path === renamingPath}
							<RenameField initialName={entry.name} onCommit={onRenameCommit} onCancel={onRenameCancel} />
						{:else}
							<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.sm, color: colors.text }}>
								{entry.name}
							</Text>
						{/if}
						{#each columns as column (column.id)}
							<Text style={cellLook(column.width, column.isRightAligned, column.isMonospace)}>
								{column.valueOf(entry)}
							</Text>
						{/each}
					</View>
				{/snippet}
			</EntrySlot>
		{/snippet}
	</List>
</View>
