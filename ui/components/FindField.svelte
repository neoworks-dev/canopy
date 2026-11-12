<!--
	Finding an entry by typing part of its name. The cursor jumps to the nearest
	match in the chosen direction as the query is typed, and the query stays
	available afterwards so the listing can step through the other matches. Behind
	a prompt the letters are free to be commands, and the query is visible while
	it is being typed.
-->
<script lang="ts">
	import { onMount, tick } from "svelte";
	import { Icon, Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { magnifyingGlass } from "../icons";
	import { nextMatchIndex, type FindDirection } from "../lib/finder";
	import { colors, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";

	let {
		query = $bindable(""),
		names,
		focusIndex,
		direction,
		onMatch,
		onClose,
	}: {
		query?: string;
		names: string[];
		focusIndex: number;
		direction: FindDirection;
		onMatch: (index: number) => void;
		onClose: (accepted: boolean) => void;
	} = $props();

	let input = $state<BareInput>();
	let hasMatch = $state(true);

	// The field exists once the frame it was added in is done, so focus waits for it.
	onMount(async () => {
		await tick();
		input?.focus();
	});

	function jumpToMatch() {
		const match = nextMatchIndex(names, query, focusIndex, direction, true);
		hasMatch = match >= 0 || query === "";
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

	function placeholder(): string {
		if (direction === "backward") {
			return "Find backwards in this folder";
		}
		return "Find in this folder";
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
		placeholder={placeholder()}
		oninput={jumpToMatch}
		onkey={handleKey}
		onblur={() => onClose(true)}
	/>
	<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>Esc to cancel</Text>
</View>
