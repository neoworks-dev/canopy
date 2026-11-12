<!--
	The listing as rows under a header of sortable columns. Only the rows near the
	window are built, so a folder of a hundred thousand entries costs what a
	screenful does. The design system's ListRow carries a title and a subtitle
	and no column tracks, so a row with selectable state and configurable columns
	is built here from the same tokens.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { List, Text, View, merge, type Layout } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import { entryRowHeight, entryValueColor, markerWidth, type EntryState } from "../lib/entryStyles";
	import type { EntryViewProps } from "../lib/entryList";
	import { gitMarkFor } from "../lib/gitStatus";
	import type { SortKey, SortOrder } from "../lib/sorting";
	import { colors, space, textSize } from "../theme";
	import EntryLine from "./EntryLine.svelte";
	import EntrySlot from "./EntrySlot.svelte";
	import RenameField from "./RenameField.svelte";
	import SortHeader from "./SortHeader.svelte";

	const headerHeight = 24;
	// The glyph before the name and the git mark after it, which the header leaves room for.
	const glyphColumnWidth = 14;
	const markColumnWidth = 12;

	let {
		entries,
		columns,
		selectedPaths,
		cutPaths,
		copiedPaths,
		gitStatuses,
		focusIndex,
		renamingPath,
		onPress,
		dragPathsFor,
		onDragStart,
		onDragEnd,
		onDropOnFolder,
		canDropInto,
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

	function whileRenaming(entry: filesystem.Entry, field: Snippet): Snippet | undefined {
		if (entry.path !== renamingPath) {
			return undefined;
		}
		return field;
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

	function cellLook(width: number, isRightAligned: boolean, isMonospace: boolean, rowState: EntryState) {
		return merge(
			{ shrink: 0, width, truncate: true, fontSize: textSize.xs, color: entryValueColor(rowState) },
			isRightAligned && { textAlign: "right" },
			isMonospace && { fontFamily: "mono" },
		);
	}

	function stateOf(entry: filesystem.Entry, index: number, isDropOver: boolean): EntryState {
		return {
			isSelected: selectedPaths.has(entry.path),
			isFocused: index === focusIndex,
			isDropOver,
			isCut: cutPaths.has(entry.path),
			isCopied: copiedPaths.has(entry.path),
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
			paddingRight: space[2],
			fontSize: textSize["2xs"],
			uppercase: true,
			color: colors.faint,
		}}
	>
		<View style={{ width: space[1] + markerWidth + space[2] + glyphColumnWidth, shrink: 0 }} />
		<SortHeader label="Name" sortKey="name" width={0} isRightAligned={false} {order} {onSort} />
		<View style={{ width: markColumnWidth, shrink: 0 }} />
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
				{canDropInto}
			>
				{#snippet children(isDropOver)}
					{@const rowState = stateOf(entry, index, isDropOver)}
					{#snippet renameField()}
						<RenameField initialName={entry.name} onCommit={onRenameCommit} onCancel={onRenameCancel} />
					{/snippet}
					<EntryLine
						{entry}
						state={rowState}
						name={whileRenaming(entry, renameField)}
						onpress={(event) => onPress(index, event)}
					>
						{#snippet trailing()}
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
							{#each columns as column (column.id)}
								<Text style={cellLook(column.width, column.isRightAligned, column.isMonospace, rowState)}>
									{column.valueOf(entry)}
								</Text>
							{/each}
						{/snippet}
					</EntryLine>
				{/snippet}
			</EntrySlot>
		{/snippet}
	</List>
</View>
