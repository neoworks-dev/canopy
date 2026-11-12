/**
 * What one pane reports about itself to the chrome around it: the path bar, the
 * status strip and the "when" clauses that decide which commands apply.
 */
import type { filesystem } from "../backend/models";

export type PaneStatus = {
	path: string;
	parentPath: string;
	entryCount: number;
	/** Entries the current filter is keeping off screen. */
	hiddenCount: number;
	selected: filesystem.Entry[];
	/** The entry under the cursor, whatever is marked; null in an empty listing. */
	hovered: filesystem.Entry | null;
	/** Where the cursor is among the entries shown, -1 when there is none. */
	cursorIndex: number;
	/** Yazi's mode: normal, or sweeping a range that selects or unsets. */
	mode: "normal" | "select" | "unset";
	/** The listing is what the pane is showing, rather than trash, tags or an error. */
	isListActive: boolean;
	/** The branch the folder is on, empty outside a working tree. */
	branch: string;
};

export const emptyPaneStatus: PaneStatus = {
	path: "",
	parentPath: "",
	entryCount: 0,
	hiddenCount: 0,
	selected: [],
	hovered: null,
	cursorIndex: -1,
	mode: "normal",
	isListActive: false,
	branch: "",
};
