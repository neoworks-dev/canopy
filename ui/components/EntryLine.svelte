<!--
	One entry as a line of a Yazi listing: the marker at the left edge, a glyph
	and the name in the colour of its kind, and whatever the linemode adds on
	the right. The hovered line is a solid bar in that colour. Rows, Miller
	columns and the folder preview all draw entries with it, so they cannot
	drift apart.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { Text, View, merge, type PointerEvent } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import { entryHover, entryLook, kindColor, markerColor, markerWidth, type EntryState } from "../lib/entryStyles";
	import { classifyEntry } from "../lib/fileKind";
	import { space, textSize } from "../theme";
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

	const kind = $derived(classifyEntry(entry));
	const nameColor = $derived(kindColor(kind));

	// Folders stand out by weight as well as colour, as Yazi's default theme has them.
	function nameWeight(shown: filesystem.Entry): "medium" | "normal" {
		if (shown.isDir) {
			return "medium";
		}
		return "normal";
	}
</script>

<View
	style={merge({ flex: 1, direction: "row", align: "center", gap: space[2], paddingRight: space[2] }, entryLook(state, kind))}
	hover={entryHover(state)}
	{onpress}
>
	<View style={{ width: markerWidth, height: "full", shrink: 0, background: markerColor(state) }} />
	<EntryIcon {entry} size="glyph" color={nameColor} />
	{#if name}
		{@render name()}
	{:else}
		<Text
			style={{
				flex: 1,
				minWidth: 0,
				truncate: true,
				fontSize: textSize.sm,
				fontWeight: nameWeight(entry),
				color: nameColor,
			}}
		>
			{entry.name}
		</Text>
	{/if}
	{@render trailing?.()}
</View>
