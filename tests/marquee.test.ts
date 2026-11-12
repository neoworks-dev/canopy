import { describe, expect, test } from "bun:test";
import {
	clampToBounds,
	exceedsThreshold,
	indexAtPoint,
	indexesInGrid,
	indexesWithin,
	isIndexInView,
	overlaps,
	rectangleBetween,
	toContentPoint,
	toViewportPoint,
	emptyGridLayout,
	type GridLayout,
	type Rectangle,
} from "../ui/lib/marquee";

const tiles: Rectangle[] = [
	{ left: 0, top: 0, width: 100, height: 40 },
	{ left: 0, top: 50, width: 100, height: 40 },
	{ left: 0, top: 100, width: 100, height: 40 },
];

describe("whether an entry is already in view", () => {
	const rows: GridLayout = { ...emptyGridLayout, columns: 1, cellWidth: 300, cellHeight: 30, hitWidth: 300, hitHeight: 30 };

	test("the first row of an unscrolled list is in view, so revealing it moves nothing", () => {
		expect(isIndexInView(0, rows, 0, 300)).toBe(true);
	});

	test("a row scrolled past, or only half shown, is not", () => {
		expect(isIndexInView(0, rows, 30, 300)).toBe(false);
		expect(isIndexInView(10, rows, 0, 300)).toBe(false);
		expect(isIndexInView(9, rows, 0, 300)).toBe(true);
	});

	test("a grid counts rows of several entries", () => {
		const grid: GridLayout = { ...rows, columns: 4, cellHeight: 100 };
		expect(isIndexInView(7, grid, 0, 200)).toBe(true);
		expect(isIndexInView(8, grid, 0, 200)).toBe(false);
	});

	test("a list with no height yet has nothing in view", () => {
		expect(isIndexInView(0, rows, 0, 0)).toBe(false);
	});
});

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

describe("entries on a grid", () => {
	// Three columns of 100x40 tiles, 10 apart, starting 20 pixels down.
	const grid: GridLayout = {
		columns: 3,
		cellWidth: 110,
		cellHeight: 50,
		hitWidth: 100,
		hitHeight: 40,
		left: 0,
		top: 20,
	};

	test("a band over one tile covers that tile", () => {
		const band: Rectangle = { left: 10, top: 30, width: 20, height: 10 };

		expect(indexesInGrid(band, 0, grid, 7)).toEqual([0]);
	});

	test("a band reaches the tiles across and down", () => {
		const band: Rectangle = { left: 90, top: 60, width: 140, height: 20 };

		expect(indexesInGrid(band, 0, grid, 7)).toEqual([0, 1, 2, 3, 4, 5]);
	});

	test("the gap between tiles is not part of either", () => {
		const band: Rectangle = { left: 102, top: 30, width: 4, height: 4 };

		expect(indexesInGrid(band, 0, grid, 7)).toEqual([]);
	});

	test("the scroll offset moves the band over the content", () => {
		const band: Rectangle = { left: 10, top: 10, width: 20, height: 10 };

		expect(indexesInGrid(band, 120, grid, 12)).toEqual([6]);
	});

	test("a tile past the last entry is not covered", () => {
		const band: Rectangle = { left: 0, top: 70, width: 300, height: 10 };

		expect(indexesInGrid(band, 0, grid, 4)).toEqual([3]);
	});

	test("a layout nothing has measured yet covers nothing", () => {
		expect(indexesInGrid({ left: 0, top: 0, width: 50, height: 50 }, 0, emptyGridLayout, 10)).toEqual([]);
	});

	test("a point finds the tile it is over, or none", () => {
		expect(indexAtPoint({ x: 120, y: 30 }, 0, grid, 7)).toBe(1);
		expect(indexAtPoint({ x: 105, y: 30 }, 0, grid, 7)).toBe(-1);
		expect(indexAtPoint({ x: 10, y: 5 }, 0, grid, 7)).toBe(-1);
	});
});
