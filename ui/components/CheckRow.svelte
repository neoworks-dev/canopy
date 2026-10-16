<!--
	A checkbox with its label, where pressing anywhere on the row toggles it.
	The design system's Checkbox is just the box.
-->
<script lang="ts">
	import { Button, Text } from "@neoworks/svelte-gpui/ui";
	import { colors, radius, space, textSize } from "../theme";
	import Checkbox from "./design/Checkbox.svelte";

	let {
		checked = $bindable(false),
		label,
		size = "md",
		onchange = undefined,
	}: {
		checked?: boolean;
		label: string;
		size?: "sm" | "md";
		onchange?: (checked: boolean) => void;
	} = $props();

	function toggle() {
		checked = !checked;
		onchange?.(checked);
	}
</script>

<Button
	style={{ direction: "row", align: "center", gap: space[2], padding: space[2], radius: radius.md }}
	hover={{ background: colors.hover }}
	onpress={toggle}
>
	<Checkbox bind:checked {size} {onchange} />
	<Text style={{ fontSize: textSize.sm, color: colors.text }}>{label}</Text>
</Button>
