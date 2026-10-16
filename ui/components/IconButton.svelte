<!--
	A small icon-only button that stays quiet until hovered: the design system's
	Button is a full-height control, which is too loud for the tools inside a
	row. `dimmed` hides it while keeping its room, for tools that appear when
	the row is hovered.
-->
<script lang="ts">
	import { Button, Icon, Tooltip, merge, type PointerEvent } from "@neoworks/svelte-gpui/ui";
	import { colors, duration, radius, space } from "../theme";

	let {
		icon,
		size = 14,
		title = "",
		dimmed = false,
		disabled = false,
		onpress,
	}: {
		/** SVG markup. */
		icon: string;
		size?: number;
		/** Shown as a tooltip. */
		title?: string;
		dimmed?: boolean;
		disabled?: boolean;
		onpress: (event: PointerEvent) => void;
	} = $props();

	const look = $derived(
		merge(
			{
				shrink: 0,
				align: "center",
				justify: "center",
				padding: space[1],
				radius: radius.sm,
				color: colors.faint,
				transition: duration.fast,
			},
			dimmed && { opacity: 0 },
		),
	);
</script>

{#snippet button()}
	<Button
		style={look}
		hover={{ background: colors.hover, color: colors.text }}
		disabledStyle={{ color: colors.lineFaint }}
		{disabled}
		{onpress}
	>
		<Icon svg={icon} {size} />
	</Button>
{/snippet}

{#if title}
	<Tooltip text={title} anchorStyle={{ alignSelf: "center" }}>
		{@render button()}
	</Tooltip>
{:else}
	{@render button()}
{/if}
