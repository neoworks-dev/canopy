/**
 * What the details pane says and draws about a sound file. The numbers arrive
 * from ffprobe, so a field the container did not carry comes through as zero and
 * is left out rather than shown as nothing.
 */
import type { audio } from "../backend/models";

export type DetailRow = { label: string; value: string };

/** Clock time, with the hours only on something long enough to have them. */
export function formatDuration(seconds: number): string {
	if (!Number.isFinite(seconds) || seconds < 0) {
		return "0:00";
	}
	const whole = Math.floor(seconds);
	if (whole < 3600) {
		return `${Math.floor(whole / 60)}:${padded(whole % 60)}`;
	}
	return `${Math.floor(whole / 3600)}:${padded(Math.floor(whole / 60) % 60)}:${padded(whole % 60)}`;
}

function padded(value: number): string {
	return value.toString().padStart(2, "0");
}

export function audioDetailRows(details: audio.Details): DetailRow[] {
	const rows: DetailRow[] = [];
	appendRow(rows, "Length", durationLabel(details.duration));
	appendRow(rows, "Bitrate", bitrateLabel(details.bitrate));
	appendRow(rows, "Sample rate", sampleRateLabel(details.sampleRate));
	appendRow(rows, "Channels", channelLabel(details.channels));
	appendRow(rows, "Codec", details.codec.toUpperCase());
	return rows;
}

function appendRow(rows: DetailRow[], label: string, value: string) {
	if (value === "") {
		return;
	}
	rows.push({ label, value });
}

function durationLabel(seconds: number): string {
	if (seconds <= 0) {
		return "";
	}
	return formatDuration(seconds);
}

function bitrateLabel(bitsPerSecond: number): string {
	if (bitsPerSecond <= 0) {
		return "";
	}
	return `${Math.round(bitsPerSecond / 1000)} kbps`;
}

function sampleRateLabel(hertz: number): string {
	if (hertz <= 0) {
		return "";
	}
	return `${round(hertz / 1000, 1)} kHz`;
}

function round(value: number, places: number): number {
	const scale = 10 ** places;
	return Math.round(value * scale) / scale;
}

// The common counts have names people recognise faster than a number.
function channelLabel(count: number): string {
	const names: Record<number, string> = { 1: "Mono", 2: "Stereo", 6: "5.1", 8: "7.1" };
	if (count <= 0) {
		return "";
	}
	const named = names[count];
	if (named) {
		return named;
	}
	return `${count} channels`;
}

/** Where in the track a click at this offset lands, as a fraction of its length. */
export function seekFraction(offsetX: number, width: number): number {
	if (width <= 0) {
		return 0;
	}
	return clampFraction(offsetX / width);
}

export function playedFraction(currentTime: number, duration: number): number {
	if (!Number.isFinite(duration) || duration <= 0) {
		return 0;
	}
	return clampFraction(currentTime / duration);
}

function clampFraction(value: number): number {
	return Math.min(1, Math.max(0, value));
}

/**
 * The peaks the pane has room to draw. The backend sends more than any pane is
 * wide, so this picks the loudest of each run rather than dropping the ones in
 * between, which would make a quiet passage look like silence.
 */
export function waveformBars(waveform: number[], barCount: number): number[] {
	if (waveform.length === 0 || barCount <= 0) {
		return [];
	}
	const bars: number[] = [];
	for (let index = 0; index < barCount; index++) {
		bars.push(loudestBetween(waveform, index, barCount));
	}
	return bars;
}

function loudestBetween(waveform: number[], bar: number, barCount: number): number {
	const start = Math.floor((bar * waveform.length) / barCount);
	const end = Math.max(start + 1, Math.floor(((bar + 1) * waveform.length) / barCount));
	return Math.max(...waveform.slice(start, end));
}
