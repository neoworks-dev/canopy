<!--
	One entry as a line of a listing: the marker for what is selected, copied or
	cut, a glyph and the name, and whatever the linemode adds on the right. Rows,
	Miller columns and the folder preview all draw entries with it, so they
	cannot drift apart.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { Text, View, merge, type PointerEvent } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import {
		entryHover,
		entryLook,
		entryGlyphColor,
		entryNameColor,
		markerColor,
		markerHeight,
		markerWidth,
		type EntryState,
	} from "../lib/entryStyles";
	import { radius, space, textSize } from "../theme";
	import EntryIcon from "./EntryIcon.svelte";

	let {
		entry,
		state,
		onpress = undefined,
		name = undefined,
		trailing = undefined,
	}: {
		entry: filesystem.Entry;
		state: EntryState;
		onpress?: (event: PointerEvent) => void;
		/** Stands in for the name, as a rename field does. */
		name?: Snippet;
		/** The linemode's values, drawn after the name. */
		trailing?: Snippet;
	} = $props();

	const nameColor = $derived(entryNameColor(state));
</script>

<View
	style={merge(
		{ flex: 1, direction: "row", align: "center", gap: space[2], paddingLeft: space[1], paddingRight: space[2], radius: radius.md },
		entryLook(state),
	)}
	hover={entryHover(state)}
	{onpress}
>
	<View style={{ width: markerWidth, height: markerHeight, shrink: 0, radius: radius.full, background: markerColor(state) }} />
	<EntryIcon {entry} size="glyph" color={entryGlyphColor(state)} />
	{#if name}
		{@render name()}
	{:else}
		<Text
			style={{
				flex: 1,
				minWidth: 0,
				truncate: true,
				fontSize: textSize.sm,
				color: nameColor,
			}}
		>
			{entry.name}
		</Text>
	{/if}
	{@render trailing?.()}
</View>
