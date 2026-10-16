<!--
	One column header. A column the backend cannot sort by is the plain label it
	would be anyway, rather than a control that does nothing. A `width` of 0 is a
	column that takes the room the others leave.
-->
<script lang="ts">
	import { Button, Icon, Text, View, merge } from "@neoworks/svelte-gpui/ui";
	import { caretDownBold, caretUpBold } from "../icons";
	import { sortDirectionOf, type SortKey, type SortOrder } from "../lib/sorting";
	import { colors, duration, radius, space } from "../theme";

	let {
		label,
		sortKey,
		width,
		isRightAligned,
		order,
		onSort,
	}: {
		label: string;
		sortKey: SortKey | "";
		width: number;
		isRightAligned: boolean;
		order: SortOrder;
		onSort: (key: SortKey) => void;
	} = $props();

	const direction = $derived(directionOf(sortKey, order));
	const cell = $derived(cellLook(width, isRightAligned));

	function directionOf(key: SortKey | "", current: SortOrder): "ascending" | "descending" | "" {
		if (!key) {
			return "";
		}
		return sortDirectionOf(current, key);
	}

	function cellLook(columnWidth: number, alignsRight: boolean) {
		return merge(
			{ direction: "row", shrink: 0 },
			columnWidth > 0 && { width: columnWidth },
			columnWidth === 0 && { flex: 1, minWidth: 0 },
			alignsRight && { justify: "end" },
		);
	}

	function sortByThisColumn() {
		if (!sortKey) {
			return;
		}
		onSort(sortKey);
	}

	// The column the listing is sorted by reads as the active one.
	function headerColor(current: string): string {
		if (current === "") {
			return colors.faint;
		}
		return colors.text;
	}

	function caret(): string {
		if (direction === "ascending") {
			return caretUpBold;
		}
		return caretDownBold;
	}
</script>

<View style={cell}>
	{#if sortKey}
		<Button
			style={{
				direction: "row",
				align: "center",
				gap: space[1],
				minWidth: 0,
				radius: radius.xs,
				color: headerColor(direction),
				transition: duration.fast,
			}}
			hover={{ color: colors.text }}
			onpress={sortByThisColumn}
		>
			<Text style={{ truncate: true }}>{label}</Text>
			{#if direction !== ""}
				<Icon svg={caret()} size={10} style={{ shrink: 0 }} />
			{/if}
		</Button>
	{:else}
		<Text style={{ truncate: true }}>{label}</Text>
	{/if}
</View>
