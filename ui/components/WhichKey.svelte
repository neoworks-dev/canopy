<!--
	Yazi's which-key: after the first key of a sequence, the keys that can come
	next and what each does, in columns along the bottom of the window. It only
	shows; the window's key handler still takes the next key.
-->
<script lang="ts">
	import { Text, View } from "@neoworks/svelte-gpui/ui";
	import type { HelpRow } from "../lib/help";
	import { colors, radius, shadow, space, textSize } from "../theme";

	const keyWidth = 64;
	const cellWidth = 260;

	let { prefix, rows }: { prefix: string; rows: HelpRow[] } = $props();
</script>

<View style={{ position: "absolute", left: space[2], right: space[2], bottom: 40, align: "center" }}>
	<View
		style={{
			gap: space[2],
			padding: space[3],
			radius: radius.lg,
			borderWidth: 1,
			borderColor: colors.lineFaint,
			background: colors.elevated,
			shadow: shadow.lg,
		}}
	>
		<Text style={{ fontFamily: "mono", fontSize: textSize.xs, color: colors.faint }}>{prefix} …</Text>
		<View style={{ direction: "row", wrap: true, columnGap: space[4], rowGap: space[1] }}>
			{#each rows as row (row.keys + row.command)}
				<View style={{ direction: "row", align: "baseline", gap: space[2], width: cellWidth }}>
					<Text style={{ width: keyWidth, shrink: 0, fontFamily: "mono", fontSize: textSize.sm, fontWeight: "bold", color: colors.text }}>
						{row.keys}
					</Text>
					<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.sm, color: colors.muted }}>
						{row.title}
					</Text>
				</View>
			{/each}
		</View>
	</View>
</View>
