<!--
	A sidebar group under a quiet caps label that folds. The design system's
	SectionHeader is a page heading with no collapse, so this is built from the
	same tokens.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
	import { caretDownBold, caretRightBold } from "../icons";
	import { colors, duration, space, textSize } from "../theme";

	let {
		title,
		isCollapsed = false,
		onToggle,
		children,
	}: {
		title: string;
		isCollapsed?: boolean;
		onToggle: () => void;
		children: Snippet;
	} = $props();

	// Styles cannot rotate, so the caret is swapped rather than turned.
	function caret(): string {
		if (isCollapsed) {
			return caretRightBold;
		}
		return caretDownBold;
	}
</script>

<View>
	<Button
		style={{
			direction: "row",
			align: "center",
			gap: space[1],
			paddingX: space[2],
			paddingY: space[1],
			fontSize: textSize.xs,
			fontWeight: "medium",
			color: colors.dim,
			transition: duration.fast,
		}}
		hover={{ color: colors.text }}
		onpress={onToggle}
	>
		<Icon svg={caret()} size={10} />
		<Text style={{ shrink: 0 }}>{title}</Text>
	</Button>

	{#if !isCollapsed}
		<View>
			{@render children()}
		</View>
	{/if}
</View>
