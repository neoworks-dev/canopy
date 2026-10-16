/** How an entry in a listing is drawn, which a row, a tile and a column share. */
import type { Style } from "@neoworks/svelte-gpui/ui";
import { colors, duration } from "../theme";

/** The height of one row: the icon, and the ring that marks the focused entry. */
export const entryRowHeight = 40;

/** The ring is a border of this width, transparent until the entry needs it. */
const ringWidth = 2;

export type EntryState = {
	isSelected: boolean;
	isFocused: boolean;
	/** A drag from elsewhere is over this folder. */
	isDropOver: boolean;
	/** A cut is holding the entry. */
	isCut: boolean;
	/** Git is not tracking the entry on purpose. */
	isMuted: boolean;
};

/** The paint an entry has in the state it is in, laid over the shape of the row or tile. */
export function entryLook(state: EntryState): Style {
	return {
		borderWidth: ringWidth,
		borderColor: ringColor(state),
		background: backgroundOf(state),
		opacity: opacityOf(state),
		transition: duration.fast,
	};
}

/** What the pointer does to an entry: a selected one already stands out. */
export function entryHover(state: EntryState): Style {
	if (state.isSelected || state.isDropOver) {
		return {};
	}
	return { background: colors.hover };
}

function ringColor(state: EntryState): string {
	if (state.isDropOver) {
		return colors.action;
	}
	if (state.isFocused) {
		return colors.lineStrong;
	}
	return "transparent";
}

function backgroundOf(state: EntryState): string {
	if (state.isDropOver) {
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
