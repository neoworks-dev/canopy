<!-- The design system's section header: a title, and an optional action at the end. -->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { Text, View } from "@neoworks/svelte-gpui/ui";
	import { colors, space, textSize } from "../../theme";

	let { title, action = undefined }: { title: string; action?: Snippet } = $props();
</script>

<View
	style={{ direction: "row", align: "center", justify: "between", paddingX: space[1], paddingBottom: space[2] }}
>
	<Text style={{ fontSize: textSize.sm, fontWeight: "semibold", color: colors.text }}>{title}</Text>
	{#if action}
		<View style={{ fontSize: textSize.xs }}>{@render action()}</View>
	{/if}
</View>
