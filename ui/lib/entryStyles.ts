/**
 * How an entry in a listing is drawn, which a row, a tile and a column share.
 * The look is Yazi's: names coloured by what they are, the hovered entry as a
 * solid bar in its own colour, and a marker at the left edge for what is
 * selected, copied or cut.
 */
import type { Style } from "@neoworks/svelte-gpui/ui";
import type { FileKind } from "./fileKind";
import { colors, duration } from "../theme";

/** The height of one row: a line of text with a little air, as a terminal row has. */
export const entryRowHeight = 26;

/** The width of the marker at a row's left edge. */
export const markerWidth = 3;

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

const colorsByKind: Record<FileKind, string> = {
	folder: colors.blue,
	image: colors.amber,
	audio: colors.pink,
	video: colors.pink,
	archive: colors.red,
	pdf: colors.cyan,
	code: colors.text,
	text: colors.text,
	file: colors.text,
};

const tintsByKind: Record<FileKind, string> = {
	folder: colors.blueSoft,
	image: colors.amberSoft,
	audio: colors.pinkSoft,
	video: colors.pinkSoft,
	archive: colors.redSoft,
	pdf: colors.cyanSoft,
	code: colors.hover,
	text: colors.hover,
	file: colors.hover,
};

/** The colour a name is drawn in, by what kind of entry it is. */
export function kindColor(kind: FileKind): string {
	return colorsByKind[kind];
}

/** The paint an entry has in the state it is in, laid over the shape of the row or tile. */
export function entryLook(state: EntryState, kind: FileKind): Style {
	return {
		background: backgroundOf(state, kind),
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

function backgroundOf(state: EntryState, kind: FileKind): string {
	if (state.isDropOver) {
		return colors.hover;
	}
	if (state.isFocused) {
		return tintsByKind[kind];
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
