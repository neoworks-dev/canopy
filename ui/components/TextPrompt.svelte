<!--
	A one-line question near the top of the window, as Yazi asks for a tab name
	or a shell command: Enter answers, Escape or a press outside dismisses.
-->
<script lang="ts">
	import { untrack } from "svelte";
	import { Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { colors, space, textSize } from "../theme";
	import PromptPanel from "./PromptPanel.svelte";

	let {
		icon,
		placeholder,
		hint = "",
		initialValue = "",
		onSubmit,
		onClose,
	}: {
		/** SVG markup. */
		icon: string;
		placeholder: string;
		hint?: string;
		initialValue?: string;
		onSubmit: (value: string) => void;
		onClose: () => void;
	} = $props();

	let value = $state(untrack(() => initialValue));

	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			onSubmit(value);
		}
	}
</script>

<PromptPanel {icon} {placeholder} bind:value onkey={handleKey} {onClose}>
	{#if hint}
		<View style={{ paddingX: space[2.5], paddingY: space[1] }}>
			<Text style={{ fontSize: textSize["2xs"], color: colors.faint }}>{hint}</Text>
		</View>
	{/if}
</PromptPanel>
