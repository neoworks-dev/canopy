<!--
	Every key and what it does, searchable by either, as Yazi's help menu. Enter
	runs the highlighted row's command. Built on the same prompt shell as the
	command palette; the palette lists commands, this lists keys.
-->
<script lang="ts">
	import { untrack } from "svelte";
	import { List, Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { question } from "../icons";
	import { filterHelpRows, type HelpRow } from "../lib/help";
	import { colors, duration, radius, space, textSize } from "../theme";
	import PromptPanel from "./PromptPanel.svelte";

	const rowHeight = 30;
	const listMaxHeight = 420;
	const keyColumnWidth = 120;

	let {
		rows,
		onRun,
		onClose,
	}: {
		rows: HelpRow[];
		onRun: (command: string) => void;
		onClose: () => void;
	} = $props();

	let query = $state("");
	let activeIndex = $state(0);
	let list = $state<List<HelpRow>>();

	const matches = $derived(filterHelpRows(rows, query));
	const listHeight = $derived(Math.min(matches.length * rowHeight, listMaxHeight));

	$effect(() => revealRow(activeIndex));

	function revealRow(index: number) {
		untrack(() => list?.scrollToIndex(index, { align: "nearest" }));
	}

	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			runActive();
			return;
		}
		moveActive(event);
	}

	function runActive() {
		const match = matches[activeIndex];
		if (!match) {
			return;
		}
		onRun(match.command);
	}

	function moveActive(event: KeyEvent) {
		const step = arrowStep(event);
		if (step === 0 || matches.length === 0) {
			return;
		}
		event.preventDefault();
		activeIndex = (activeIndex + step + matches.length) % matches.length;
	}

	// The field has the focus, so j and k type; Ctrl+n and Ctrl+p move instead.
	function arrowStep(event: KeyEvent): number {
		if (event.key === "ArrowDown" || (event.ctrl && event.key === "n")) {
			return 1;
		}
		if (event.key === "ArrowUp" || (event.ctrl && event.key === "p")) {
			return -1;
		}
		return 0;
	}

	function highlightOnHover(hovered: boolean, index: number) {
		if (hovered) {
			activeIndex = index;
		}
	}

	function rowBackground(index: number): string {
		if (index === activeIndex) {
			return colors.hover;
		}
		return "transparent";
	}
</script>

<PromptPanel
	icon={question}
	placeholder="Filter keys"
	bind:value={query}
	oninput={() => (activeIndex = 0)}
	onkey={handleKey}
	{onClose}
>
	{#if matches.length === 0}
		<Text style={{ padding: space[2], fontSize: textSize.xs, color: colors.dim }}>No key matches</Text>
	{:else}
		<List bind:this={list} items={matches} rowHeight={rowHeight} style={{ height: listHeight }}>
			{#snippet row(match, index)}
				<View
					style={{
						direction: "row",
						align: "center",
						gap: space[2],
						height: rowHeight,
						paddingX: space[2],
						radius: radius.md,
						background: rowBackground(index),
						transition: duration.fast,
					}}
					onhover={(hovered) => highlightOnHover(hovered, index)}
					onpress={() => onRun(match.command)}
				>
					<Text
						style={{ width: keyColumnWidth, shrink: 0, fontFamily: "mono", fontSize: textSize.xs, color: colors.action }}
					>
						{match.keys}
					</Text>
					<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.sm, color: colors.muted }}>
						{match.title}
					</Text>
				</View>
			{/snippet}
		</List>
	{/if}
</PromptPanel>
