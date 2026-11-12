<!--
	A file's text as lines of coloured runs. A file of any size is one long run of
	tokens, which `layoutLines` turns into lines; the list under it only builds the
	lines on screen. Highlighted source keeps its lines as written and scrolls
	sideways; plain text wraps at the width it has.
-->
<script lang="ts">
	import { List, Scroll, Text, View, type Layout, type ScrollPosition } from "@neoworks/svelte-gpui/ui";
	import type { highlight } from "../backend/models";
	import { layoutLines, longestLine, type CodeRun } from "../lib/codeCanvas";
	import { styleOfTokenClass } from "../lib/highlightColors";
	import { colors, radius, space, textSize } from "../theme";

	const padding = space[2];
	const fontSize = textSize["2xs"];
	const lineHeight = 16;
	// Every glyph of a monospaced face is the same width, which is what lets a
	// column be a character offset: JetBrains Mono is six tenths of its size wide.
	const glyphWidth = fontSize * 0.6;

	let {
		tokens,
		isWrapped = false,
	}: {
		tokens: highlight.Token[];
		/** Long lines wrap instead of scrolling sideways. */
		isWrapped?: boolean;
	} = $props();

	// The columns need a height to fill, which a scrolling row around them cannot give.
	let height = $state(0);
	let width = $state(0);

	const lines = $derived(layoutLines(tokens));
	const contentWidth = $derived(Math.max(width, longestLine(lines) * glyphWidth + padding * 2));

	const textLook = {
		fontFamily: "mono",
		fontSize,
		lineHeight,
		color: colors.text,
	} as const;

	let list = $state<List<CodeRun[]>>();
	// The line at the top of the window. Wrapped lines are taller than one row,
	// so there it is an estimate, which is all a seek needs.
	let topLine = 0;

	/** Moves the window by a number of lines, down when positive. */
	export function seek(lineCount: number) {
		const target = Math.max(0, Math.min(lines.length - 1, topLine + lineCount));
		topLine = target;
		list?.scrollToIndex(target, { align: "start" });
	}

	function followScroll(position: ScrollPosition) {
		topLine = Math.floor(position.y / lineHeight);
	}

	function measured(layout: Layout) {
		width = layout.width;
		height = layout.height;
	}

	function runLook(run: CodeRun) {
		return styleOfTokenClass(run.className);
	}
</script>

{#snippet lineText(line: CodeRun[])}
	<Text pre style={textLook}>
		{#each line as run (run.column)}
			<Text style={runLook(run)}>{run.text}</Text>
		{/each}
	</Text>
{/snippet}

<View
	style={{ flex: 1, minHeight: 0, radius: radius.lg, background: colors.raised, overflow: "hidden" }}
	onresize={measured}
>
	{#if isWrapped}
		<List bind:this={list} items={lines} onscroll={followScroll} style={{ flex: 1, minHeight: 0 }}>
			{#snippet row(line)}
				<View style={{ paddingX: padding }}>{@render lineText(line)}</View>
			{/snippet}
		</List>
	{:else}
		<Scroll horizontal style={{ flex: 1, minHeight: 0 }}>
			<View style={{ width: contentWidth, height }}>
				<List
					bind:this={list}
					items={lines}
					rowHeight={lineHeight}
					onscroll={followScroll}
					style={{ flex: 1, minHeight: 0 }}
				>
					{#snippet row(line)}
						<View style={{ paddingX: padding, height: lineHeight }}>{@render lineText(line)}</View>
					{/snippet}
				</List>
			</View>
		</Scroll>
	{/if}
</View>
