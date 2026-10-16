/**
 * Keyboard movement over the entry list. The grid renders several entries per
 * row, so up and down step by the number of rendered columns rather than by one;
 * the row view is the same rules with a single column.
 *
 * Movements are named rather than keyed, because which key produces which one is
 * the keymap's business.
 */
export type Movement =
	| "down"
	| "up"
	| "left"
	| "right"
	| "pageDown"
	| "pageUp"
	| "first"
	| "last";

export type MovementContext = {
	total: number;
	/** Entries per rendered row: 1 in the row view. */
	columns: number;
	/** Entries one page movement covers. */
	pageSize: number;
};

/** Index to move to, or -1 when the movement leads nowhere. */
export function targetIndexFor(
	movement: Movement,
	currentIndex: number,
	context: MovementContext,
): number {
	if (context.total === 0) {
		return -1;
	}
	const step = stepFor(movement, context);
	if (step === 0) {
		return edgeIndexFor(movement, context.total);
	}
	if (currentIndex < 0) {
		return 0;
	}
	return clampIndex(currentIndex + step, context.total);
}

function stepFor(movement: Movement, context: MovementContext): number {
	if (movement === "down") {
		return context.columns;
	}
	if (movement === "up") {
		return -context.columns;
	}
	return pageStepFor(movement, context);
}

function pageStepFor(movement: Movement, context: MovementContext): number {
	if (movement === "pageDown") {
		return Math.max(1, context.pageSize);
	}
	if (movement === "pageUp") {
		return -Math.max(1, context.pageSize);
	}
	return horizontalStepFor(movement, context);
}

/** Left and right only mean something where a row holds more than one entry. */
function horizontalStepFor(movement: Movement, context: MovementContext): number {
	if (context.columns < 2) {
		return 0;
	}
	if (movement === "right") {
		return 1;
	}
	if (movement === "left") {
		return -1;
	}
	return 0;
}

function edgeIndexFor(movement: Movement, total: number): number {
	if (movement === "first") {
		return 0;
	}
	if (movement === "last") {
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
