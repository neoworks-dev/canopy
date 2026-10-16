/**
 * Windowing maths for the listing, kept free of the DOM so a directory with
 * 100k entries mounts only the rows on screen. Both views lay entries out on a
 * uniform grid, so a window is arithmetic on row heights rather than a
 * measurement per entry.
 */

/** What a view reports about its own layout, measured from one mounted entry. */
export type ListMetrics = {
	/** Height of one row or tile, the gap below it included. */
	itemHeight: number;
	/** Entries per row: 1 in the row view, however many tiles fit in the grid. */
	columns: number;
	/** Height rendered above the first entry, e.g. the row view's column header. */
	leadingHeight: number;
};

/** Where the viewport currently sits over the entries. */
export type Viewport = {
	scrollTop: number;
	height: number;
};

/** The slice to mount, with the space the unmounted entries stand in for. */
export type ListWindow = {
	firstIndex: number;
	/** Exclusive, so entries.slice(firstIndex, endIndex) is the mounted slice. */
	endIndex: number;
	topPadding: number;
	bottomPadding: number;
};

export const unmeasuredMetrics: ListMetrics = { itemHeight: 0, columns: 1, leadingHeight: 0 };

/**
 * Whether a fresh measurement says anything new. A view re-measures whenever the
 * window it was handed changes, so storing every report would feed the window
 * back into itself.
 */
export function sameMetrics(current: ListMetrics, reported: ListMetrics): boolean {
	return (
		current.itemHeight === reported.itemHeight &&
		current.columns === reported.columns &&
		current.leadingHeight === reported.leadingHeight
	);
}

/**
 * Rows kept mounted above and below the viewport, so a scroll of a row or two
 * does not wait for a re-render.
 */
const overscanRows = 4;

/**
 * How many entries to mount before a view has reported its metrics. Enough to
 * fill any viewport and to measure from, far short of a directory that stalls.
 */
const probeCount = 60;

export function listWindow(
	metrics: ListMetrics,
	total: number,
	viewport: Viewport,
): ListWindow {
	if (metrics.itemHeight <= 0) {
		return { firstIndex: 0, endIndex: Math.min(total, probeCount), topPadding: 0, bottomPadding: 0 };
	}
	const rowCount = Math.ceil(total / metrics.columns);
	const firstRow = firstVisibleRow(metrics, viewport, rowCount);
	const lastRow = lastVisibleRow(metrics, viewport, firstRow, rowCount);
	return {
		firstIndex: firstRow * metrics.columns,
		endIndex: Math.min(total, lastRow * metrics.columns),
		topPadding: firstRow * metrics.itemHeight,
		bottomPadding: (rowCount - lastRow) * metrics.itemHeight,
	};
}

function firstVisibleRow(metrics: ListMetrics, viewport: Viewport, rowCount: number): number {
	const above = (viewport.scrollTop - metrics.leadingHeight) / metrics.itemHeight;
	const row = Math.floor(above) - overscanRows;
	return clamp(row, 0, rowCount);
}

function lastVisibleRow(
	metrics: ListMetrics,
	viewport: Viewport,
	firstRow: number,
	rowCount: number,
): number {
	const covered = Math.ceil(viewport.height / metrics.itemHeight) + 1;
	return clamp(firstRow + covered + 2 * overscanRows, 0, rowCount);
}

/** Offset of an entry's row from the top of the scrolled content. */
export function offsetOfIndex(metrics: ListMetrics, index: number): number {
	const row = Math.floor(index / metrics.columns);
	return metrics.leadingHeight + row * metrics.itemHeight;
}

/**
 * Scroll offset that brings an entry into view, leaving it where it is when it
 * already is. Replaces scrollIntoView, which cannot reach an unmounted entry.
 */
export function scrollTopRevealing(
	metrics: ListMetrics,
	index: number,
	viewport: Viewport,
): number {
	if (metrics.itemHeight <= 0 || index < 0) {
		return viewport.scrollTop;
	}
	const top = offsetOfIndex(metrics, index);
	if (top < viewport.scrollTop) {
		return top;
	}
	const bottom = top + metrics.itemHeight;
	if (bottom > viewport.scrollTop + viewport.height) {
		return bottom - viewport.height;
	}
	return viewport.scrollTop;
}

/** Rows a page moves by while a view has not reported its geometry yet. */
const unmeasuredPageRows = 10;

/** Entries one PageUp or PageDown covers. */
export function pageSize(metrics: ListMetrics, viewport: Viewport): number {
	if (metrics.itemHeight <= 0) {
		return unmeasuredPageRows * metrics.columns;
	}
	const rows = Math.floor(viewport.height / metrics.itemHeight);
	return Math.max(1, rows) * metrics.columns;
}

function clamp(value: number, lowest: number, highest: number): number {
	return Math.min(Math.max(value, lowest), highest);
}
