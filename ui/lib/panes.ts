/**
 * Widths of the panes flanking the listing. Kept free of the DOM so the limits
 * and the drag arithmetic stay testable.
 */
export type PaneSide = "left" | "right";

export type PaneWidths = {
	sidebar: number;
	details: number;
};

/** How far one arrow key press moves a gutter. */
export const paneResizeStep = 16;

/** The width of the gutter a resizer sits in, in pixels. */
export const paneGutterWidth = 5;

export const minimumPaneWidth = 160;
// The places rail stays compact; the details pane holds previews, so it may grow.
const maximumPaneWidth: Record<PaneSide, number> = { left: 420, right: 640 };

export const defaultPaneWidths: PaneWidths = { sidebar: 208, details: 320 };

export function maximumWidthFor(side: PaneSide): number {
	return maximumPaneWidth[side];
}

/** Whole pixels: a drag reports fractional positions, and the settings file stores widths as integers. */
export function clampPaneWidth(side: PaneSide, width: number): number {
	return Math.min(maximumPaneWidth[side], Math.max(minimumPaneWidth, Math.round(width)));
}

/**
 * The gutter is dragged, not the pane, so the pane on the left of it grows with
 * a rightward drag and the one on the right grows with a leftward drag.
 */
export function paneWidthAfterDrag(side: PaneSide, startWidth: number, delta: number): number {
	if (side === "left") {
		return clampPaneWidth(side, startWidth + delta);
	}
	return clampPaneWidth(side, startWidth - delta);
}

/** A stored width that is missing or unusable falls back to the default. */
export function paneWidthFrom(side: PaneSide, stored: unknown, fallback: number): number {
	if (typeof stored !== "number" || !Number.isFinite(stored) || stored <= 0) {
		return fallback;
	}
	return clampPaneWidth(side, stored);
}
