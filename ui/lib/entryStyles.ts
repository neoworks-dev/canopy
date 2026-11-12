/**
 * How an entry in a listing is drawn, which a row, a tile and a column share:
 * a neutral highlight for the hovered entry, and a marker before the glyph for
 * what is selected, copied or cut, the one place colour carries meaning.
 */
import type { Style } from "@neoworks/svelte-gpui/ui";
import { colors, duration } from "../theme";

/** The height of one row: a line of text with room around it to aim at. */
export const entryRowHeight = 30;

/** The marker before a row's glyph: a short rounded bar. */
export const markerWidth = 3;
export const markerHeight = 14;

export type EntryState = {
	isSelected: boolean;
	isFocused: boolean;
	/** A drag from elsewhere is over this folder. */
	isDropOver: boolean;
	/** A cut is holding the entry. */
	isCut: boolean;
	/** A copy is holding the entry. */
	isCopied: boolean;
	/** Git is not tracking the entry on purpose. */
	isMuted: boolean;
};

/** The colour of an entry's name. */
export function entryNameColor(state: EntryState): string {
	if (state.isFocused || state.isSelected) {
		return colors.text;
	}
	return colors.muted;
}

/** The colour of the glyph before a name: quieter than the name, so the name leads. */
export function entryGlyphColor(state: EntryState): string {
	if (state.isFocused || state.isSelected) {
		return colors.muted;
	}
	return colors.dim;
}

/** The paint an entry has in the state it is in, laid over the shape of the row or tile. */
export function entryLook(state: EntryState): Style {
	return {
		background: backgroundOf(state),
		opacity: opacityOf(state),
		transition: duration.fast,
	};
}


/** The colour of the values beside a name: brighter on the hovered bar. */
export function entryValueColor(state: EntryState): string {
	if (state.isFocused) {
		return colors.muted;
	}
	return colors.dim;
}

/** What the pointer does to an entry: the hovered one already stands out. */
export function entryHover(state: EntryState): Style {
	if (state.isFocused || state.isDropOver) {
		return {};
	}
	return { background: colors.hover };
}

/** The marker at the left edge, or transparent when the entry is held by nothing. */
export function markerColor(state: EntryState): string {
	if (state.isCut) {
		return colors.red;
	}
	if (state.isCopied) {
		return colors.green;
	}
	if (state.isSelected) {
		return colors.amber;
	}
	return "transparent";
}

function backgroundOf(state: EntryState): string {
	if (state.isDropOver || state.isFocused) {
		return colors.hover;
	}
	if (state.isSelected) {
		return colors.raised;
	}
	return "transparent";
}

function opacityOf(state: EntryState): number {
	if (state.isCut) {
		return 0.5;
	}
	if (state.isMuted) {
		return 0.6;
	}
	return 1;
}
