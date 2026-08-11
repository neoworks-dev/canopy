import { describe, expect, test } from "bun:test";
import {
	clampToBounds,
	exceedsThreshold,
	indexesWithin,
	overlaps,
	rectangleBetween,
	toContentPoint,
	toViewportPoint,
	type Rectangle,
} from "../frontend/src/lib/marquee";

const tiles: Rectangle[] = [
	{ left: 0, top: 0, width: 100, height: 40 },
	{ left: 0, top: 50, width: 100, height: 40 },
	{ left: 0, top: 100, width: 100, height: 40 },
];

describe("content coordinates", () => {
	const bounds: Rectangle = { left: 100, top: 50, width: 200, height: 100 };

	test("reads a point on screen as an offset into the scrolled content", () => {
		expect(toContentPoint({ x: 150, y: 80 }, bounds, 400)).toEqual({ x: 150, y: 430 });
	});

	test("puts a content point back where it now sits on screen", () => {
		expect(toViewportPoint({ x: 150, y: 430 }, bounds, 400)).toEqual({ x: 150, y: 80 });
	});

	test("moves an anchor up the screen as the listing scrolls down under it", () => {
		const anchor = toContentPoint({ x: 150, y: 80 }, bounds, 400);
		expect(toViewportPoint(anchor, bounds, 460)).toEqual({ x: 150, y: 20 });
	});
});

describe("clampToBounds", () => {
	const bounds: Rectangle = { left: 100, top: 50, width: 200, height: 100 };

	test("leaves a point inside the bounds alone", () => {
		expect(clampToBounds({ x: 150, y: 80 }, bounds)).toEqual({ x: 150, y: 80 });
	});

	test("pulls a point past the far edges back onto them", () => {
		expect(clampToBounds({ x: 900, y: 900 }, bounds)).toEqual({ x: 300, y: 150 });
	});

	test("pulls a point before the near edges back onto them", () => {
		expect(clampToBounds({ x: -40, y: 0 }, bounds)).toEqual({ x: 100, y: 50 });
	});
});

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
