<!-- The path as a row of folder names, each a button that goes there. -->
<script lang="ts">
	import { Button, Icon, Scroll, View, merge } from "@neoworks/svelte-gpui/ui";
	import { caretRight } from "../icons";
	import { toBreadcrumbs } from "../lib/format";
	import { colors, duration, radius, space, textSize } from "../theme";

	let {
		path,
		onNavigate,
	}: {
		path: string;
		onNavigate: (path: string) => void;
	} = $props();

	const crumbs = $derived(toBreadcrumbs(path));

	function crumbLook(isLast: boolean) {
		return merge(
			{
				shrink: 0,
				paddingX: space[2],
				paddingY: space[1],
				radius: radius.sm,
				fontSize: textSize.sm,
				color: colors.muted,
				transition: duration.fast,
			},
			isLast && { color: colors.text, fontWeight: "medium" },
		);
	}
</script>

<Scroll horizontal style={{ minWidth: 0, shrink: 1 }}>
	<View style={{ direction: "row", align: "center", gap: space[0.5] }}>
		{#each crumbs as crumb, index (crumb.path)}
			{#if index > 0}
				<Icon svg={caretRight} size={12} color={colors.faint} style={{ shrink: 0 }} />
			{/if}
			<Button
				style={crumbLook(index === crumbs.length - 1)}
				hover={{ background: colors.hover, color: colors.text }}
				onpress={() => onNavigate(crumb.path)}
			>
				{crumb.label}
			</Button>
		{/each}
	</View>
</Scroll>
