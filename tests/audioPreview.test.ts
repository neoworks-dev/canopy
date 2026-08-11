import { describe, expect, test } from "bun:test";
import {
	audioDetailRows,
	formatDuration,
	mediaUrlFor,
	playedFraction,
	seekFraction,
	waveformBars,
} from "../frontend/src/lib/audioPreview";
import type { audio } from "../frontend/wailsjs/go/models";

function details(overrides: Partial<audio.Details> = {}): audio.Details {
	return {
		duration: 0,
		bitrate: 0,
		channels: 0,
		sampleRate: 0,
		codec: "",
		title: "",
		artist: "",
		album: "",
		waveform: [],
		...overrides,
	} as audio.Details;
}

function labelled(rows: Array<{ label: string; value: string }>, label: string): string {
	const row = rows.find((candidate) => candidate.label === label);
	if (!row) {
		return "";
	}
	return row.value;
}

describe("the address a track plays from", () => {
	test("the path is escaped, so a space or an ampersand survives it", () => {
		expect(mediaUrlFor("/music/two words & more.mp3")).toBe(
			"/media?path=%2Fmusic%2Ftwo%20words%20%26%20more.mp3",
		);
	});
});

describe("clock time", () => {
	test("a short track is minutes and seconds", () => {
		expect(formatDuration(213)).toBe("3:33");
	});

	test("seconds below ten keep their leading zero", () => {
		expect(formatDuration(65)).toBe("1:05");
	});

	test("an hour long recording grows an hours field", () => {
		expect(formatDuration(3725)).toBe("1:02:05");
	});

	test("a length that is not a number reads as zero", () => {
		expect(formatDuration(Number.NaN)).toBe("0:00");
	});
});

describe("the rows describing a track", () => {
	test("a field the container did not carry is left out", () => {
		expect(audioDetailRows(details())).toEqual([]);
	});

	test("the bitrate is rounded to kbps", () => {
		expect(labelled(audioDetailRows(details({ bitrate: 320000 })), "Bitrate")).toBe("320 kbps");
	});

	test("the sample rate keeps one decimal, which is what 44.1 kHz needs", () => {
		expect(labelled(audioDetailRows(details({ sampleRate: 44100 })), "Sample rate")).toBe(
			"44.1 kHz",
		);
	});

	test("the common channel counts have names", () => {
		expect(labelled(audioDetailRows(details({ channels: 2 })), "Channels")).toBe("Stereo");
		expect(labelled(audioDetailRows(details({ channels: 6 })), "Channels")).toBe("5.1");
	});

	test("an uncommon channel count is counted out", () => {
		expect(labelled(audioDetailRows(details({ channels: 3 })), "Channels")).toBe("3 channels");
	});
});

describe("seeking", () => {
	test("a click halfway along lands halfway through", () => {
		expect(seekFraction(50, 100)).toBe(0.5);
	});

	test("a click outside the track is clamped to it", () => {
		expect(seekFraction(-10, 100)).toBe(0);
		expect(seekFraction(150, 100)).toBe(1);
	});

	test("a track of no width cannot be sought", () => {
		expect(seekFraction(50, 0)).toBe(0);
	});

	test("progress through a track of unknown length is none", () => {
		expect(playedFraction(30, 0)).toBe(0);
	});

	test("progress past the end is still the end", () => {
		expect(playedFraction(120, 100)).toBe(1);
	});
});

describe("fitting a waveform to the pane", () => {
	test("more peaks than bars keep the loudest of each run", () => {
		expect(waveformBars([0.1, 0.9, 0.2, 0.3], 2)).toEqual([0.9, 0.3]);
	});

	test("fewer peaks than bars still fill every bar", () => {
		expect(waveformBars([0.4, 0.8], 4)).toHaveLength(4);
	});

	test("a track with no waveform draws no bars", () => {
		expect(waveformBars([], 10)).toEqual([]);
	});

	test("a pane with no room draws no bars", () => {
		expect(waveformBars([0.5], 0)).toEqual([]);
	});
});
