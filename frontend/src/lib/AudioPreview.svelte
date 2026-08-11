<script lang="ts">
	import MusicNotesIcon from "phosphor-svelte/lib/MusicNotesIcon";
	import PauseIcon from "phosphor-svelte/lib/PauseIcon";
	import PlayIcon from "phosphor-svelte/lib/PlayIcon";
	import type { audio } from "../../wailsjs/go/models";
	import {
		audioDetailRows,
		formatDuration,
		mediaUrlFor,
		playedFraction,
		seekFraction,
		waveformBars,
	} from "./audioPreview";

	// A sound file has nothing to draw of its own, so the pane shows its cover
	// art, what the container says about it, and the shape of the sound as a
	// waveform that doubles as the seek bar.
	let {
		path,
		name,
		details,
		coverUrl,
	}: {
		path: string;
		name: string;
		details: audio.Details;
		coverUrl: string;
	} = $props();

	// One bar every this many pixels, which stays legible at any pane width.
	const barPitch = 3;
	const barWidth = 2;

	let element = $state<HTMLAudioElement>();
	let canvas = $state<HTMLCanvasElement>();
	let canvasWidth = $state(0);
	let canvasHeight = $state(0);
	let currentTime = $state(0);
	let paused = $state(true);
	// The stylesheet decides what the two halves of the waveform look like, so the
	// colours are read back off elements carrying those classes.
	let playedProbe = $state<HTMLSpanElement>();
	let remainingProbe = $state<HTMLSpanElement>();

	const rows = $derived(audioDetailRows(details));
	const played = $derived(playedFraction(currentTime, details.duration));

	$effect(() => {
		details.waveform;
		played;
		canvasWidth;
		canvasHeight;
		draw();
	});

	function togglePlayback() {
		if (!element) {
			return;
		}
		if (!element.paused) {
			element.pause();
			return;
		}
		element.play().catch(() => undefined);
	}

	function playbackLabel(): string {
		if (paused) {
			return "Play";
		}
		return "Pause";
	}

	function seekTo(event: MouseEvent & { currentTarget: HTMLDivElement }) {
		if (!element || details.duration <= 0) {
			return;
		}
		const fraction = seekFraction(event.offsetX, event.currentTarget.clientWidth);
		element.currentTime = fraction * details.duration;
	}

	function draw() {
		const context = canvas?.getContext("2d");
		if (!context) {
			return;
		}
		prepareCanvas(context);
		drawBars(context, waveformBars(details.waveform, Math.floor(canvasWidth / barPitch)));
	}

	// A canvas is sized in device pixels and drawn in CSS ones, and resizing it
	// clears everything the context was told.
	function prepareCanvas(context: CanvasRenderingContext2D) {
		const ratio = window.devicePixelRatio;
		context.canvas.width = Math.ceil(canvasWidth * ratio);
		context.canvas.height = Math.ceil(canvasHeight * ratio);
		context.scale(ratio, ratio);
	}

	function drawBars(context: CanvasRenderingContext2D, bars: number[]) {
		const playedColour = measuredColour(playedProbe);
		const remainingColour = measuredColour(remainingProbe);
		const playedBars = Math.round(bars.length * played);
		bars.forEach((peak, index) => {
			context.fillStyle = barColour(index < playedBars, playedColour, remainingColour);
			context.fillRect(index * barPitch, barTop(peak), barWidth, barHeight(peak));
		});
	}

	function barColour(isPlayed: boolean, playedColour: string, remainingColour: string): string {
		if (isPlayed) {
			return playedColour;
		}
		return remainingColour;
	}

	// The smallest bar is still a line, so silence reads as part of the track
	// rather than as a gap in it.
	function barHeight(peak: number): number {
		return Math.max(1, peak * canvasHeight);
	}

	function barTop(peak: number): number {
		return (canvasHeight - barHeight(peak)) / 2;
	}

	function measuredColour(probe: HTMLSpanElement | undefined): string {
		if (!probe) {
			return "currentColor";
		}
		return getComputedStyle(probe).color;
	}
</script>

<div class="flex min-h-0 flex-col gap-2">
	<div class="flex shrink-0 items-center justify-center rounded-lg bg-raised p-2">
		{#if coverUrl}
			<img src={coverUrl} alt="Cover art for {name}" class="max-h-48 rounded-md object-contain" />
		{:else}
			<MusicNotesIcon size={48} class="text-faint" />
		{/if}
	</div>

	{#if details.title || details.artist}
		<div class="shrink-0">
			{#if details.title}
				<p class="truncate text-sm font-medium text-default">{details.title}</p>
			{/if}
			{#if details.artist}
				<p class="truncate text-xs text-muted">{details.artist}</p>
			{/if}
			{#if details.album}
				<p class="truncate text-2xs text-faint">{details.album}</p>
			{/if}
		</div>
	{/if}

	<div class="flex shrink-0 items-center gap-2">
		<button
			type="button"
			onclick={togglePlayback}
			aria-label={playbackLabel()}
			class="shrink-0 rounded-md bg-raised p-2 text-muted transition duration-fast hover:bg-hover hover:text-default"
		>
			{#if paused}
				<PlayIcon size={14} weight="fill" />
			{:else}
				<PauseIcon size={14} weight="fill" />
			{/if}
		</button>

		<!--
			The waveform is the seek bar: clicking it is the same gesture as clicking
			a progress track, so it carries the slider role rather than a button one.
		-->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			bind:clientWidth={canvasWidth}
			bind:clientHeight={canvasHeight}
			onclick={seekTo}
			role="slider"
			tabindex="-1"
			aria-label="Seek"
			aria-valuemin={0}
			aria-valuemax={Math.round(details.duration)}
			aria-valuenow={Math.round(currentTime)}
			class="h-10 min-w-0 flex-1 cursor-pointer rounded-md bg-raised"
		>
			<canvas bind:this={canvas} class="h-full w-full"></canvas>
		</div>

		<span class="shrink-0 text-2xs text-faint tabular-nums">
			{formatDuration(currentTime)} / {formatDuration(details.duration)}
		</span>
	</div>

	{#if rows.length > 0}
		<dl class="flex shrink-0 flex-col gap-2 rounded-lg bg-raised p-2">
			{#each rows as row (row.label)}
				<div class="flex items-baseline justify-between gap-2">
					<dt class="text-2xs tracking-caps text-faint uppercase">{row.label}</dt>
					<dd class="truncate text-xs text-muted">{row.value}</dd>
				</div>
			{/each}
		</dl>
	{/if}

	<audio bind:this={element} bind:currentTime bind:paused src={mediaUrlFor(path)} preload="metadata"
	></audio>

	<span bind:this={playedProbe} class="hidden text-action"></span>
	<span bind:this={remainingProbe} class="hidden text-faint"></span>
</div>
