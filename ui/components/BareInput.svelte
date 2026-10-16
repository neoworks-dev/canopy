<!--
	A text field with no box of its own, for sitting inside a row that draws the
	border, an icon and the buttons around it.
-->
<script lang="ts">
	import { Input, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { colors, textSize } from "../theme";

	let {
		value = $bindable(""),
		placeholder = "",
		oninput = undefined,
		onsubmit = undefined,
		onkey = undefined,
		onblur = undefined,
	}: {
		value?: string;
		placeholder?: string;
		oninput?: (value: string) => void;
		onsubmit?: (value: string) => void;
		onkey?: (event: KeyEvent) => void;
		onblur?: () => void;
	} = $props();

	let input = $state<Input>();

	export function focus() {
		input?.focus();
	}

	export function select() {
		input?.select();
	}
</script>

<Input
	bind:this={input}
	bind:value
	{placeholder}
	{oninput}
	{onsubmit}
	{onkey}
	{onblur}
	style={{ flex: 1, minWidth: 0, fontSize: textSize.sm, color: colors.text, background: "transparent" }}
/>
