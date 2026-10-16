<!--
	Finding an entry by typing its name used to happen on the listing itself,
	which meant every letter key belonged to it. Behind a prompt the letters are
	free to be commands, and the query is visible while it is being typed.
-->
<script lang="ts">
	import { onMount } from "svelte";
	import { Icon, Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { magnifyingGlass } from "../icons";
	import { typeAheadMatch } from "../lib/typeAhead";
	import { colors, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";

	let {
		names,
		focusIndex,
		onMatch,
		onClose,
	}: {
		names: string[];
		focusIndex: number;
		onMatch: (index: number) => void;
		onClose: (accepted: boolean) => void;
	} = $props();

	let query = $state("");
	let input = $state<BareInput>();
	let hasMatch = $state(true);

	onMount(() => input?.focus());

	function jumpToMatch() {
		const match = typeAheadMatch(names, query.trim().toLowerCase(), focusIndex);
		hasMatch = match >= 0 || query.trim() === "";
		if (match < 0) {
			return;
		}
		onMatch(match);
	}

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

	function borderColor(): string {
		if (hasMatch) {
			return colors.lineFaint;
		}
		return colors.red;
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
		borderColor: borderColor(),
		background: colors.surface,
	}}
>
	<Icon svg={magnifyingGlass} size={14} color={colors.faint} style={{ shrink: 0 }} />
	<BareInput
		bind:this={input}
		bind:value={query}
		placeholder="Find in this folder"
		oninput={jumpToMatch}
		onkey={handleKey}
		onblur={() => onClose(true)}
	/>
	<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>Esc to cancel</Text>
</View>
