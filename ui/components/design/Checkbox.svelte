<!-- The design system's checkbox: a small box that fills with the action colour. -->
<script lang="ts">
	import { Button, Icon, merge } from "@neoworks/svelte-gpui/ui";
	import { checkBold } from "../../icons";
	import { colors, radius } from "../../theme";

	type Size = "sm" | "md";

	let {
		checked = $bindable(false),
		size = "md",
		disabled = false,
		onchange = undefined,
	}: {
		checked?: boolean;
		size?: Size;
		disabled?: boolean;
		onchange?: (checked: boolean) => void;
	} = $props();

	const boxSizes: Record<Size, number> = { sm: 14, md: 16 };
	const iconSizes: Record<Size, number> = { sm: 10, md: 12 };

	const look = $derived(
		merge(
			{
				size: boxSizes[size],
				radius: radius.xs,
				borderWidth: 1,
				borderColor: colors.line,
				background: colors.input,
				align: "center",
				justify: "center",
				transition: 120,
			},
			checked && { borderColor: colors.action, background: colors.action },
		),
	);

	function toggle() {
		checked = !checked;
		onchange?.(checked);
	}
</script>

<Button style={look} focus={{ borderColor: colors.lineStrong }} {disabled} onpress={toggle}>
	{#if checked}
		<Icon svg={checkBold} size={iconSizes[size]} color={colors.actionForeground} />
	{/if}
</Button>
