<!--
	The design system's button (`@neoworks-dev/ui` Button) on svelte-gpui primitives:
	the same variants, sizes and round icon-only form. `icon` is SVG markup.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { Button, Icon, merge, type PointerEvent, type Style } from "@neoworks/svelte-gpui/ui";
	import { colors, radius, space, textSize } from "../../theme";

	type Variant = "primary" | "surface" | "ghost" | "success" | "danger";
	type Size = "sm" | "md" | "lg";

	let {
		variant = "surface",
		size = "md",
		icon = undefined,
		disabled = false,
		full = false,
		round = false,
		style = undefined,
		onpress = undefined,
		children = undefined,
	}: {
		variant?: Variant;
		size?: Size;
		icon?: string;
		disabled?: boolean;
		full?: boolean;
		/** A circle sized to the height, for an icon-only button. */
		round?: boolean;
		style?: Style;
		onpress?: (event: PointerEvent) => void;
		children?: Snippet;
	} = $props();

	const variantStyles: Record<Variant, { rest: Style; hover: Style; active: Style }> = {
		primary: {
			rest: { background: colors.action, color: colors.actionForeground, borderColor: "transparent" },
			hover: { background: colors.actionHover },
			active: { background: colors.actionHover },
		},
		surface: {
			rest: { background: colors.raised, color: colors.text, borderColor: colors.line },
			hover: { background: colors.hover, borderColor: colors.lineStrong },
			active: { background: colors.lineStrong },
		},
		ghost: {
			rest: { background: "transparent", color: colors.muted, borderColor: "transparent" },
			hover: { background: colors.hover, color: colors.text },
			active: { background: colors.raised, color: colors.text },
		},
		success: {
			rest: { background: colors.green, color: "#000000", borderColor: "transparent" },
			hover: { opacity: 0.9 },
			active: { opacity: 0.8 },
		},
		danger: {
			rest: { background: colors.redSoft, color: colors.red, borderColor: colors.redBorder },
			hover: { background: colors.redHover },
			active: { background: colors.redHover },
		},
	};

	const sizeStyles: Record<Size, Style> = {
		sm: { height: 36, paddingX: space[3], fontSize: textSize.xs, gap: space[1.5], radius: radius.sm },
		md: { height: 44, paddingX: space[4], fontSize: textSize.sm, gap: space[2], radius: radius.md },
		lg: { height: 56, paddingX: space[5], fontSize: textSize.md, gap: space[2], radius: radius.lg },
	};

	const roundStyles: Record<Size, Style> = {
		sm: { size: 36, fontSize: textSize.xs, radius: radius.full },
		md: { size: 44, fontSize: textSize.sm, radius: radius.full },
		lg: { size: 56, fontSize: textSize.md, radius: radius.full },
	};

	const iconSizes: Record<Size, number> = { sm: 14, md: 16, lg: 20 };

	const base: Style = {
		direction: "row",
		align: "center",
		justify: "center",
		borderWidth: 1,
		fontWeight: "semibold",
		nowrap: true,
		shrink: 0,
		transition: 150,
	};

	function shapeFor(current: Size, isRound: boolean): Style {
		if (isRound) {
			return roundStyles[current];
		}
		return sizeStyles[current];
	}

	const look = $derived(
		merge(base, shapeFor(size, round), variantStyles[variant].rest, full && { width: "full" }, style),
	);
</script>

<Button
	style={look}
	hover={variantStyles[variant].hover}
	active={variantStyles[variant].active}
	{disabled}
	disabledStyle={{ opacity: 0.5, cursor: "not-allowed" }}
	{onpress}
>
	{#if icon}
		<Icon svg={icon} size={iconSizes[size]} />
	{/if}
	{@render children?.()}
</Button>
