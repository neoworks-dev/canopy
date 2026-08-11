import { describe, expect, test } from "bun:test";
import {
	exceedsThreshold,
	indexesWithin,
	overlaps,
	rectangleBetween,
	type Rectangle,
} from "../frontend/src/lib/marquee";

const tiles: Rectangle[] = [
	{ left: 0, top: 0, width: 100, height: 40 },
	{ left: 0, top: 50, width: 100, height: 40 },
	{ left: 0, top: 100, width: 100, height: 40 },
];

describe("rectangleBetween", () => {
	test("normalises a drag towards the top left", () => {
		expect(rectangleBetween({ x: 120, y: 90 }, { x: 20, y: 10 })).toEqual({
			left: 20,
			top: 10,
			width: 100,
			height: 80,
		});
	});
});

describe("exceedsThreshold", () => {
	test("a click-sized rectangle is not a rubber band", () => {
		expect(exceedsThreshold({ left: 0, top: 0, width: 2, height: 3 })).toBe(false);
	});

	test("a real drag is", () => {
		expect(exceedsThreshold({ left: 0, top: 0, width: 2, height: 40 })).toBe(true);
	});
});

describe("overlaps", () => {
	test("touching edges count as overlapping", () => {
		const band = { left: 100, top: 0, width: 10, height: 40 };
		expect(overlaps(band, tiles[0])).toBe(true);
	});

	test("a gap does not", () => {
		const band = { left: 101, top: 0, width: 10, height: 40 };
		expect(overlaps(band, tiles[0])).toBe(false);
	});
});

describe("indexesWithin", () => {
	test("reports the covered entries in listing order", () => {
		const band = { left: 10, top: 30, width: 20, height: 80 };
		expect(indexesWithin(band, tiles)).toEqual([0, 1, 2]);
	});

	test("a band between two entries covers neither", () => {
		const band = { left: 10, top: 42, width: 20, height: 4 };
		expect(indexesWithin(band, tiles)).toEqual([]);
	});
});
