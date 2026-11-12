<!--
	Narrows the listing to the entries whose names contain what is typed. The
	list follows every keystroke; Enter keeps the filter and returns to the list,
	Esc drops it.
-->
<script lang="ts">
	import { onMount } from "svelte";
	import { Icon, Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { funnel } from "../icons";
	import { colors, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";

	let {
		query = $bindable(""),
		onClose,
	}: {
		query?: string;
		onClose: (accepted: boolean) => void;
	} = $props();

	let input = $state<BareInput>();

	onMount(() => {
		input?.focus();
		input?.select();
	});

	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			event.preventDefault();
			onClose(false);
		}
		if (event.key === "Enter") {
			event.preventDefault();
			onClose(true);
		}
	}
</script>

<View
	style={{
		direction: "row",
		shrink: 0,
		align: "center",
		gap: space[2],
		paddingX: space[2.5],
		paddingY: space[1.5],
		radius: radius.sm,
		borderWidth: 1,
		borderColor: colors.lineFaint,
		background: colors.surface,
	}}
>
	<Icon svg={funnel} size={14} color={colors.faint} style={{ shrink: 0 }} />
	<BareInput
		bind:this={input}
		bind:value={query}
		placeholder="Filter this folder"
		onkey={handleKey}
		onblur={() => onClose(true)}
	/>
	<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>Enter to keep · Esc to clear</Text>
</View>
