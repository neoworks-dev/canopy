/**
 * Rubber-band selection maths, kept free of the DOM so the hit testing stays
 * testable. All rectangles are in viewport coordinates.
 */
export type Point = {
	x: number;
	y: number;
};

export type Rectangle = {
	left: number;
	top: number;
	width: number;
	height: number;
};

/** A drag shorter than this is a click, not a rubber band. */
export const marqueeThreshold = 4;

export function rectangleBetween(start: Point, current: Point): Rectangle {
	return {
		left: Math.min(start.x, current.x),
		top: Math.min(start.y, current.y),
		width: Math.abs(current.x - start.x),
		height: Math.abs(current.y - start.y),
	};
}

/**
 * Where a point on screen sits in the scrolled content, so an anchor survives
 * the listing scrolling under it.
 */
export function toContentPoint(point: Point, bounds: Rectangle, scrollTop: number): Point {
	return { x: point.x, y: point.y - bounds.top + scrollTop };
}

/** The inverse of {@link toContentPoint}, at whatever the scroll is now. */
export function toViewportPoint(point: Point, bounds: Rectangle, scrollTop: number): Point {
	return { x: point.x, y: point.y + bounds.top - scrollTop };
}

/**
 * Keeps a dragged point inside the listing, so a band that follows the pointer
 * off the list never paints over the panes around it.
 */
export function clampToBounds(point: Point, bounds: Rectangle): Point {
	return {
		x: clamp(point.x, bounds.left, bounds.left + bounds.width),
		y: clamp(point.y, bounds.top, bounds.top + bounds.height),
	};
}

function clamp(value: number, lowest: number, highest: number): number {
	return Math.min(Math.max(value, lowest), highest);
}

export function exceedsThreshold(rectangle: Rectangle): boolean {
	return rectangle.width > marqueeThreshold || rectangle.height > marqueeThreshold;
}

export function overlaps(first: Rectangle, second: Rectangle): boolean {
	if (first.left > second.left + second.width || second.left > first.left + first.width) {
		return false;
	}
	return !(first.top > second.top + second.height || second.top > first.top + first.height);
}

/** Indexes of the entries the band currently covers, in listing order. */
export function indexesWithin(band: Rectangle, itemRectangles: Rectangle[]): number[] {
	const indexes: number[] = [];
	itemRectangles.forEach((rectangle, index) => {
		if (overlaps(band, rectangle)) {
			indexes.push(index);
		}
	});
	return indexes;
}

/**
 * Where entries sit in a view that lays them out on a regular grid, which is how
 * a click or a band is matched to an entry without asking each one. A list of
 * rows is the same grid with one column.
 */
export type GridLayout = {
	columns: number;
	/** The distance from one cell to the next across and down, gap included. */
	cellWidth: number;
	cellHeight: number;
	/** The part of a cell an entry occupies, which leaves the gap out. */
	hitWidth: number;
	hitHeight: number;
	/** Where the first cell starts, in the coordinates of the view holding the entries. */
	left: number;
	top: number;
};

export const emptyGridLayout: GridLayout = {
	columns: 1,
	cellWidth: 0,
	cellHeight: 0,
	hitWidth: 0,
	hitHeight: 0,
	left: 0,
	top: 0,
};

/**
 * The entries a band covers. The band is in viewport coordinates; the layout is
 * in content coordinates, so the scroll offset joins them. Only the rows the
 * band reaches are looked at, however many entries there are.
 */
export function indexesInGrid(
	band: Rectangle,
	scrollTop: number,
	layout: GridLayout,
	total: number,
): number[] {
	if (layout.cellWidth <= 0 || layout.cellHeight <= 0 || total <= 0) {
		return [];
	}
	const content = { ...band, top: band.top + scrollTop };
	const rows = cellRange(content.top, content.height, layout.top, layout.cellHeight);
	const columns = cellRange(content.left, content.width, layout.left, layout.cellWidth);
	const indexes: number[] = [];
	for (let row = rows.first; row <= rows.last; row++) {
		for (let column = Math.max(0, columns.first); column <= Math.min(columns.last, layout.columns - 1); column++) {
			appendIfCovered(indexes, content, layout, row, column, total);
		}
	}
	return indexes;
}

function cellRange(start: number, length: number, origin: number, pitch: number) {
	return {
		first: Math.max(0, Math.floor((start - origin) / pitch)),
		last: Math.floor((start + length - origin) / pitch),
	};
}

function appendIfCovered(
	indexes: number[],
	content: Rectangle,
	layout: GridLayout,
	row: number,
	column: number,
	total: number,
) {
	const index = row * layout.columns + column;
	if (index >= total) {
		return;
	}
	const cell = {
		left: layout.left + column * layout.cellWidth,
		top: layout.top + row * layout.cellHeight,
		width: layout.hitWidth,
		height: layout.hitHeight,
	};
	if (overlaps(content, cell)) {
		indexes.push(index);
	}
}

/** The entry under a point in viewport coordinates, or -1 where there is none. */
export function indexAtPoint(point: Point, scrollTop: number, layout: GridLayout, total: number): number {
	const covered = indexesInGrid({ left: point.x, top: point.y, width: 0, height: 0 }, scrollTop, layout, total);
	if (covered.length === 0) {
		return -1;
	}
	return covered[0];
}
