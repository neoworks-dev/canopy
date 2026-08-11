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
