/**
 * Keyboard movement over the entry list. The grid renders several entries per
 * row, so up and down step by the number of rendered columns rather than by one;
 * the row view is the same rules with a single column.
 */
export type MovementContext = {
	total: number;
	/** Entries per rendered row: 1 in the row view. */
	columns: number;
	/** Entries one PageUp or PageDown covers. */
	pageSize: number;
};

/** Index to move to, or -1 when the key is not a movement key. */
export function targetIndexFor(key: string, currentIndex: number, context: MovementContext): number {
	if (context.total === 0) {
		return -1;
	}
	const step = stepFor(key, context);
	if (step === 0) {
		return edgeIndexFor(key, context.total);
	}
	if (currentIndex < 0) {
		return 0;
	}
	return clampIndex(currentIndex + step, context.total);
}

function stepFor(key: string, context: MovementContext): number {
	if (key === "ArrowDown") {
		return context.columns;
	}
	if (key === "ArrowUp") {
		return -context.columns;
	}
	if (key === "PageDown") {
		return Math.max(1, context.pageSize);
	}
	if (key === "PageUp") {
		return -Math.max(1, context.pageSize);
	}
	return horizontalStepFor(key, context);
}

/** Left and right only mean something where a row holds more than one entry. */
function horizontalStepFor(key: string, context: MovementContext): number {
	if (context.columns < 2) {
		return 0;
	}
	if (key === "ArrowRight") {
		return 1;
	}
	if (key === "ArrowLeft") {
		return -1;
	}
	return 0;
}

function edgeIndexFor(key: string, total: number): number {
	if (key === "Home") {
		return 0;
	}
	if (key === "End") {
		return total - 1;
	}
	return -1;
}

function clampIndex(index: number, total: number): number {
	if (index < 0) {
		return 0;
	}
	if (index > total - 1) {
		return total - 1;
	}
	return index;
}
