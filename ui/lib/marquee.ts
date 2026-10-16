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
