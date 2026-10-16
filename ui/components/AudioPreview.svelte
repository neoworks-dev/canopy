<!--
	A sound file has nothing to draw of its own, so the pane shows its cover art,
	what the container says about it, and the shape of the sound as a waveform.
-->
<script lang="ts">
	import { Button, Canvas, Icon, Image, Text, View, type Layout } from "@neoworks/svelte-gpui/ui";
	import type { audio } from "../backend/models";
	import { playFill } from "../icons";
	import { audioDetailRows, formatDuration, waveformBars } from "../lib/audioPreview";
	import { colors, radius, space, textSize } from "../theme";

	// One bar every this many pixels, which stays legible at any pane width.
	const barPitch = 3;
	const barWidth = 2;
	const waveformHeight = 40;

	let { details, coverUrl }: { details: audio.Details; coverUrl: string } = $props();

	let canvas = $state<Canvas>();
	let width = $state(0);

	const rows = $derived(audioDetailRows(details));
	const bars = $derived(waveformBars(waveformOf(details), Math.floor(width / barPitch)));

	$effect(() => {
		draw(canvas, bars);
	});

	// A file that was never decoded carries no waveform at all.
	function waveformOf(source: audio.Details): number[] {
		if (!source.waveform) {
			return [];
		}
		return source.waveform;
	}

	function draw(surface: Canvas | undefined, peaks: number[]) {
		if (!surface) {
			return;
		}
		surface.clear();
		peaks.forEach((peak, index) => {
			const barHeight = Math.max(1, peak * waveformHeight);
			surface.fillRect(index * barPitch, (waveformHeight - barHeight) / 2, barWidth, barHeight, colors.dim);
		});
	}
</script>

<View style={{ gap: space[2] }}>
	<!-- A track with no cover art shows none: the pane's own header already names
		 the file and carries its icon. -->
	{#if coverUrl}
		<View style={{ align: "center", padding: space[2], radius: radius.lg, background: colors.raised }}>
			<Image src={coverUrl} fit="contain" style={{ width: "full", height: 160, radius: radius.md }} />
		</View>
	{/if}

	{#if details.title || details.artist}
		<View>
			{#if details.title}
				<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
					{details.title}
				</Text>
			{/if}
			{#if details.artist}
				<Text style={{ truncate: true, fontSize: textSize.xs, color: colors.muted }}>{details.artist}</Text>
			{/if}
			{#if details.album}
				<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>{details.album}</Text>
			{/if}
		</View>
	{/if}

	<View style={{ direction: "row", align: "center", gap: space[2] }}>
		<!-- svelte-gpui: there is no audio playback yet (issue #29), so the button is
			 drawn but cannot be pressed, and the waveform is not a seek bar. -->
		<Button
			disabled
			style={{ shrink: 0, padding: space[2], radius: radius.sm, background: colors.raised, color: colors.muted }}
			disabledStyle={{ color: colors.faint }}
		>
			<Icon svg={playFill} size={14} />
		</Button>

		<View
			style={{ flex: 1, minWidth: 0, height: waveformHeight, radius: radius.sm, background: colors.raised }}
			onresize={(layout: Layout) => (width = layout.width)}
		>
			<Canvas bind:this={canvas} style={{ flex: 1 }} />
		</View>

		<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>
			{formatDuration(0)} / {formatDuration(details.duration)}
		</Text>
	</View>

	{#if rows.length > 0}
		<View style={{ gap: space[2], padding: space[2], radius: radius.lg, background: colors.raised }}>
			{#each rows as row (row.label)}
				<View style={{ direction: "row", align: "baseline", justify: "between", gap: space[2] }}>
					<Text style={{ fontSize: textSize["2xs"], uppercase: true, color: colors.faint }}>{row.label}</Text>
					<Text style={{ truncate: true, fontSize: textSize.xs, color: colors.muted }}>{row.value}</Text>
				</View>
			{/each}
		</View>
	{/if}
</View>
