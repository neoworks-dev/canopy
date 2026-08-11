<script lang="ts">
	import { untrack } from "svelte";
	import MusicNotesIcon from "phosphor-svelte/lib/MusicNotesIcon";
	import PauseIcon from "phosphor-svelte/lib/PauseIcon";
	import PlayIcon from "phosphor-svelte/lib/PlayIcon";
	import { MediaURL } from "../../wailsjs/go/main/FileService";
	import type { audio } from "../../wailsjs/go/models";
	import {
		audioDetailRows,
		formatDuration,
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
	// The address the track streams from, which the Go side mints per file.
	let mediaUrl = $state("");
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

	// The pane keeps this component across a change of selection, so a new track
	// arrives as a new path rather than as a fresh player.
	$effect(() => {
		const wanted = path;
		untrack(() => startTrack(wanted));
	});

	async function startTrack(wanted: string) {
		mediaUrl = "";
		currentTime = 0;
		paused = true;
		const address = await MediaURL(wanted);
		if (wanted !== path) {
			return;
		}
		mediaUrl = address;
	}

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

	// The element's own events rather than bindings: binding currentTime drives it
	// from an animation frame loop, which redraws the waveform sixty times a second
	// for a bar that moves a few pixels a minute.
	function rememberTime(event: Event & { currentTarget: HTMLAudioElement }) {
		currentTime = event.currentTarget.currentTime;
	}

	function rememberPlaybackState(event: Event & { currentTarget: HTMLAudioElement }) {
		paused = event.currentTarget.paused;
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
		if (!context || canvasWidth === 0) {
			return;
		}
		prepareCanvas(context);
		drawBars(context, waveformBars(details.waveform, Math.floor(canvasWidth / barPitch)));
	}

	/**
	 * A canvas is sized in device pixels and drawn in CSS ones. Assigning either
	 * dimension reallocates the drawing surface even when the value is unchanged,
	 * which is not something a redraw on every frame of playback can afford, so the
	 * size is only written when it actually differs.
	 */
	function prepareCanvas(context: CanvasRenderingContext2D) {
		const ratio = window.devicePixelRatio;
		resizeCanvas(context.canvas, Math.ceil(canvasWidth * ratio), Math.ceil(canvasHeight * ratio));
		context.setTransform(ratio, 0, 0, ratio, 0, 0);
		context.clearRect(0, 0, canvasWidth, canvasHeight);
	}

	function resizeCanvas(surface: HTMLCanvasElement, width: number, height: number) {
		if (surface.width === width && surface.height === height) {
			return;
		}
		surface.width = width;
		surface.height = height;
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

	{#if mediaUrl}
		<audio
			bind:this={element}
			ontimeupdate={rememberTime}
			onseeked={rememberTime}
			onplay={rememberPlaybackState}
			onpause={rememberPlaybackState}
			onended={rememberPlaybackState}
			src={mediaUrl}
			preload="metadata"
		></audio>
	{/if}

	<span bind:this={playedProbe} class="hidden text-action"></span>
	<span bind:this={remainingProbe} class="hidden text-faint"></span>
</div>
