<!-- The design system's card: a rounded surface with a faint line around it. -->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { View, merge, type Style } from "@neoworks/svelte-gpui/ui";
	import { colors, radius, space } from "../../theme";

	type Surface = "surface" | "elevated" | "raised";
	type Padding = "none" | "sm" | "md" | "lg";

	let {
		surface = "surface",
		padding = "md",
		style = undefined,
		children = undefined,
	}: {
		surface?: Surface;
		padding?: Padding;
		style?: Style;
		children?: Snippet;
	} = $props();

	const surfaceColors: Record<Surface, string> = {
		surface: colors.surface,
		elevated: colors.elevated,
		raised: colors.raised,
	};

	const paddings: Record<Padding, number> = { none: 0, sm: space[3], md: space[4], lg: space[5] };

	const look = $derived(
		merge(
			{
				radius: radius.lg,
				borderWidth: 1,
				borderColor: colors.lineFaint,
				background: surfaceColors[surface],
				padding: paddings[padding],
			},
			style,
		),
	);
</script>

<View style={look}>
	{@render children?.()}
</View>
