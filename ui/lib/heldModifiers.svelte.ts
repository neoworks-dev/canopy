/**
 * The modifier keys held right now, which decide whether a drag copies or
 * moves. One window-wide record, so every drop target reads the same answer.
 */
import type { DragModifiers } from "./dragDrop";

export const heldModifiers = $state<DragModifiers>({ ctrlKey: false, shiftKey: false, metaKey: false });

/** Kept current from the window's key events, presses and releases alike. */
export function rememberModifiers(event: DragModifiers) {
	heldModifiers.ctrlKey = event.ctrlKey;
	heldModifiers.shiftKey = event.shiftKey;
	heldModifiers.metaKey = event.metaKey;
}
