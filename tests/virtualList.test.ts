import { describe, expect, test } from "bun:test";
import {
	listWindow,
	offsetOfIndex,
	pageSize,
	sameMetrics,
	scrollTopRevealing,
	unmeasuredMetrics,
	type ListMetrics,
	type Viewport,
} from "../ui/lib/virtualList";

const rowMetrics: ListMetrics = { itemHeight: 20, columns: 1, leadingHeight: 16 };
const gridMetrics: ListMetrics = { itemHeight: 100, columns: 5, leadingHeight: 0 };

function viewportAt(scrollTop: number, height = 400): Viewport {
	return { scrollTop, height };
}

describe("listWindow", () => {
	test("mounts a bounded slice of a huge directory", () => {
		const mounted = listWindow(rowMetrics, 100_000, viewportAt(0));
		expect(mounted.endIndex - mounted.firstIndex).toBeLessThan(40);
	});

	test("the paddings stand in for the whole directory's height", () => {
		const mounted = listWindow(rowMetrics, 100_000, viewportAt(200_000));
		const mountedHeight = (mounted.endIndex - mounted.firstIndex) * rowMetrics.itemHeight;
		expect(mounted.topPadding + mountedHeight + mounted.bottomPadding).toBe(100_000 * 20);
	});

	test("scrolling moves the window down the listing", () => {
		const mounted = listWindow(rowMetrics, 1000, viewportAt(2016));
		expect(mounted.firstIndex).toBe(96);
		expect(mounted.topPadding).toBe(1920);
	});

	test("the window never starts before the first entry", () => {
		const mounted = listWindow(rowMetrics, 1000, viewportAt(0));
		expect(mounted.firstIndex).toBe(0);
		expect(mounted.topPadding).toBe(0);
	});

	test("a listing that fits the viewport mounts entirely", () => {
		const mounted = listWindow(rowMetrics, 10, viewportAt(0));
		expect(mounted.endIndex).toBe(10);
		expect(mounted.bottomPadding).toBe(0);
	});

	test("the window never runs past the last entry", () => {
		const mounted = listWindow(rowMetrics, 1000, viewportAt(19_800));
		expect(mounted.endIndex).toBe(1000);
		expect(mounted.bottomPadding).toBe(0);
	});

	test("a grid window starts on a row boundary", () => {
		const mounted = listWindow(gridMetrics, 10_000, viewportAt(5000));
		expect(mounted.firstIndex % gridMetrics.columns).toBe(0);
		expect(mounted.endIndex % gridMetrics.columns).toBe(0);
	});

	test("before a view has measured itself, enough entries mount to measure from", () => {
		const mounted = listWindow(unmeasuredMetrics, 100_000, viewportAt(0));
		expect(mounted.firstIndex).toBe(0);
		expect(mounted.endIndex).toBe(60);
		expect(mounted.bottomPadding).toBe(0);
	});

	test("an empty listing mounts nothing", () => {
		expect(listWindow(rowMetrics, 0, viewportAt(0))).toEqual({
			firstIndex: 0,
			endIndex: 0,
			topPadding: 0,
			bottomPadding: 0,
		});
	});
});

describe("offsetOfIndex", () => {
	test("rows stack below whatever leads the listing", () => {
		expect(offsetOfIndex(rowMetrics, 3)).toBe(76);
	});

	test("tiles on the same grid row share an offset", () => {
		expect(offsetOfIndex(gridMetrics, 5)).toBe(offsetOfIndex(gridMetrics, 9));
	});
});

describe("scrollTopRevealing", () => {
	test("an entry already in view does not scroll", () => {
		expect(scrollTopRevealing(rowMetrics, 5, viewportAt(0))).toBe(0);
	});

	test("an entry above the viewport scrolls to its top", () => {
		expect(scrollTopRevealing(rowMetrics, 10, viewportAt(1000))).toBe(216);
	});

	test("an entry below the viewport scrolls just far enough", () => {
		expect(scrollTopRevealing(rowMetrics, 100, viewportAt(0))).toBe(1636);
	});

	test("nothing is revealed while no entry has focus", () => {
		expect(scrollTopRevealing(rowMetrics, -1, viewportAt(320))).toBe(320);
	});

	test("an unmeasured view stays where it is", () => {
		expect(scrollTopRevealing(unmeasuredMetrics, 40, viewportAt(120))).toBe(120);
	});
});

describe("pageSize", () => {
	test("a page is the rows that fit, times the entries on a row", () => {
		expect(pageSize(gridMetrics, viewportAt(0))).toBe(20);
	});

	test("a viewport shorter than one row still pages by one row", () => {
		expect(pageSize(rowMetrics, viewportAt(0, 10))).toBe(1);
	});

	test("an unmeasured view still pages by more than one entry", () => {
		expect(pageSize(unmeasuredMetrics, viewportAt(0))).toBe(10);
	});
});

describe("sameMetrics", () => {
	test("a repeated measurement says nothing new", () => {
		expect(sameMetrics(rowMetrics, { ...rowMetrics })).toBe(true);
	});

	test("a changed tile count is new", () => {
		expect(sameMetrics(gridMetrics, { ...gridMetrics, columns: 4 })).toBe(false);
	});

	test("a changed row height is new", () => {
		expect(sameMetrics(rowMetrics, { ...rowMetrics, itemHeight: 24 })).toBe(false);
	});

	test("a changed header height is new", () => {
		expect(sameMetrics(rowMetrics, { ...rowMetrics, leadingHeight: 0 })).toBe(false);
	});
});
