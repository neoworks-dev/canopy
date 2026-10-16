<!--
	Every command the explorer can run right now, searchable by name. The
	registry is the same one the keymap and the context menu read, so a command
	reaches all three by being registered once.

	The context is the one captured when the palette opened, because the listing
	loses focus to the field here and a command has to run against the state the
	user was looking at when they reached for it.
-->
<script lang="ts">
	import { untrack } from "svelte";
	import { Icon, List, Text, View, type KeyEvent, type Style } from "@neoworks/svelte-gpui/ui";
	import { magnifyingGlass } from "../icons";
	import type { Command } from "../lib/commands";
	import { availableCommands, rankCommands, titleSegments, type PaletteMatch, type TitleSegment } from "../lib/palette";
	import type { WhenContext } from "../lib/whenClause";
	import { colors, duration, radius, space, textSize } from "../theme";
	import PromptPanel from "./PromptPanel.svelte";

	const rowHeight = 36;
	const rowGap = 2;
	const listMaxHeight = 384;

	let {
		commands,
		context,
		shortcutOf,
		onRun,
		onClose,
	}: {
		commands: Command[];
		context: WhenContext;
		shortcutOf: (id: string) => string;
		onRun: (id: string) => void;
		onClose: () => void;
	} = $props();

	let query = $state("");
	let activeIndex = $state(0);
	let list = $state<List<PaletteMatch>>();

	const matches = $derived(rankCommands(availableCommands(commands, context), query));
	const listHeight = $derived(Math.min(matches.length * (rowHeight + rowGap), listMaxHeight));

	// The list scrolls, so arrowing past its edge has to bring the row along.
	$effect(() => revealRow(activeIndex));

	// The list updates its own state while scrolling, which must not feed back
	// into this effect.
	function revealRow(index: number) {
		untrack(() => list?.scrollToIndex(index, { align: "nearest" }));
	}

	function handleInput() {
		activeIndex = 0;
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
		onRun(match.command.id);
	}

	function moveActive(event: KeyEvent) {
		const step = arrowStep(event.key);
		if (step === 0 || matches.length === 0) {
			return;
		}
		event.preventDefault();
		activeIndex = (activeIndex + step + matches.length) % matches.length;
	}

	function arrowStep(key: string): number {
		if (key === "ArrowDown") {
			return 1;
		}
		if (key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

	function rowBackground(index: number): string {
		if (index === activeIndex) {
			return colors.hover;
		}
		return "transparent";
	}

	function highlightOnHover(hovered: boolean, index: number) {
		if (hovered) {
			activeIndex = index;
		}
	}

	function segmentLook(segment: TitleSegment): Style {
		if (segment.matched) {
			return { color: colors.action, fontWeight: "medium" };
		}
		return {};
	}

	function titleColor(index: number): string {
		if (index === activeIndex) {
			return colors.text;
		}
		return colors.muted;
	}
</script>

<PromptPanel
	icon={magnifyingGlass}
	placeholder="Run a command"
	bind:value={query}
	oninput={handleInput}
	onkey={handleKey}
	{onClose}
>
	{#if matches.length === 0}
		<Text style={{ padding: space[2], fontSize: textSize.xs, color: colors.dim }}>No command matches</Text>
	{:else}
		<List
			bind:this={list}
			items={matches}
			rowHeight={rowHeight + rowGap}
			key={(match) => match.command.id}
			style={{ height: listHeight }}
		>
			{#snippet row(match, index)}
				<View style={{ height: rowHeight + rowGap, paddingBottom: rowGap }}>
					<View
						style={{
							direction: "row",
							align: "center",
							gap: space[2],
							height: rowHeight,
							paddingX: space[2],
							radius: radius.md,
							fontSize: textSize.sm,
							background: rowBackground(index),
							transition: duration.fast,
						}}
						onhover={(hovered) => highlightOnHover(hovered, index)}
						onpress={() => onRun(match.command.id)}
					>
						<View style={{ width: 14, shrink: 0 }}>
							{#if match.command.icon}
								<Icon svg={match.command.icon} size={14} color={colors.faint} />
							{/if}
						</View>
						<Text style={{ flex: 1, minWidth: 0, truncate: true, color: titleColor(index) }}>
							{#each titleSegments(match.command.title, match.highlights) as segment}
								<Text style={segmentLook(segment)}>{segment.text}</Text>
							{/each}
						</Text>
						<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>
							{match.command.category}
						</Text>
						{#if shortcutOf(match.command.id)}
							<View
								style={{
									shrink: 0,
									paddingX: space[1],
									paddingY: space[0.5],
									radius: radius.xs,
									borderWidth: 1,
									borderColor: colors.lineFaint,
								}}
							>
								<Text style={{ fontFamily: "mono", fontSize: textSize["2xs"], color: colors.dim }}>
									{shortcutOf(match.command.id)}
								</Text>
							</View>
						{/if}
					</View>
				</View>
			{/snippet}
		</List>
	{/if}
</PromptPanel>
