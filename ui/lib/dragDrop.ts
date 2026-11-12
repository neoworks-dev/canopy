/**
 * What a drag carries, where it may land, and what dropping it means. Kept free
 * of the DOM so the rules stay testable: the components only read the payload
 * off a DataTransfer and ask these functions.
 */
import { parentDirectoryOf } from "./format";

/** Our own drags carry this, which is how a drag from outside is told apart. */
export const internalDragType = "application/x-neoworks-paths";
export const uriListType = "text/uri-list";

export type DragModifiers = {
	ctrlKey: boolean;
	shiftKey: boolean;
	metaKey: boolean;
};

export type DragSource = "internal" | "external";

/**
 * The read side of a DataTransfer. Structural so the rules can be exercised
 * without a DOM, and so nothing here depends on the browser's own object.
 */
export type DragPayload = {
	types: readonly string[];
	getData: (type: string) => string;
};

export type DropOperation = "copy" | "move";

/** The little a drop target needs to know about the entry under the pointer. */
export type HoveredEntry = {
	path: string;
	isDir: boolean;
};

/**
 * Dragging an entry that is part of the selection drags the whole selection;
 * dragging one outside it drags only that entry, which is what makes a drag
 * from an unselected row behave like a click first.
 */
export function pathsForDrag(selectedPaths: string[], entryPath: string): string[] {
	if (selectedPaths.includes(entryPath)) {
		return selectedPaths;
	}
	return [entryPath];
}

/**
 * Moving is the default inside the explorer, the way dragging within one window
 * behaves everywhere; a drag arriving from another application copies, because
 * moving would reach into a folder the user did not open here. Ctrl forces a
 * copy and Shift forces a move either way.
 */
export function dropOperation(source: DragSource, modifiers: DragModifiers): DropOperation {
	if (modifiers.ctrlKey || modifiers.metaKey) {
		return "copy";
	}
	if (modifiers.shiftKey) {
		return "move";
	}
	if (source === "external") {
		return "copy";
	}
	return "move";
}

/**
 * Where a drop lands: onto a folder when the pointer is over one, and otherwise
 * into the folder the pane is showing. A file under the pointer is not a target
 * of its own, so dropping on one means the folder it sits in.
 */
export function dropDestination(hovered: HoveredEntry | null, directoryPath: string): string {
	if (hovered && hovered.isDir) {
		return hovered.path;
	}
	return directoryPath;
}

/**
 * Whether a drop is worth starting. A move into the folder the sources already
 * sit in changes nothing, while a copy there is a real request that the engine
 * answers with a conflict prompt.
 */
export function isAcceptableDrop(
	sources: string[],
	destination: string,
	operation: DropOperation,
): boolean {
	if (sources.length === 0 || destination === "") {
		return false;
	}
	if (sources.some((source) => swallowsDestination(source, destination))) {
		return false;
	}
	return operation === "copy" || movesAnything(sources, destination);
}

/**
 * Whether a folder takes a drag from inside the window, and so lights up under
 * it: only when dropping there would do something, given the keys held.
 */
export function acceptsInternalDrop(sources: string[], destination: string, modifiers: DragModifiers): boolean {
	return isAcceptableDrop(sources, destination, dropOperation("internal", modifiers));
}

// A folder cannot be dropped into itself or into anything it contains, which
// would otherwise ask the engine to move a tree inside its own subtree.
function swallowsDestination(source: string, destination: string): boolean {
	return destination === source || destination.startsWith(`${source}/`);
}

function movesAnything(sources: string[], destination: string): boolean {
	return sources.some((source) => parentDirectoryOf(source) !== destination);
}

export function dragSourceOf(payload: DragPayload): DragSource {
	if (payload.types.includes(internalDragType)) {
		return "internal";
	}
	return "external";
}

/**
 * The paths a drop carries. Our own drags name them outright; a drag from
 * another application is read from the URI list it published.
 */
export function readDraggedPaths(payload: DragPayload): string[] {
	const own = payload.getData(internalDragType);
	if (own !== "") {
		return parsePathList(own);
	}
	return parseUriList(payload.getData(uriListType));
}

function parsePathList(text: string): string[] {
	try {
		return asPathArray(JSON.parse(text));
	} catch {
		return [];
	}
}

function asPathArray(parsed: unknown): string[] {
	if (!Array.isArray(parsed)) {
		return [];
	}
	return parsed.filter((value) => typeof value === "string" && value !== "");
}

/** The payload other applications read, one file URI per line. */
export function toUriList(paths: string[]): string {
	return paths.map(toFileUri).join("\r\n");
}

// Each segment is encoded on its own so the separators survive, and so a name
// holding a "#" or a "?" does not turn into a fragment or a query.
function toFileUri(path: string): string {
	return `file://${path.split("/").map(encodeURIComponent).join("/")}`;
}

/**
 * Reads a text/uri-list another application wrote. Anything that is not a local
 * file is dropped rather than passed on as a path the engine cannot open.
 */
export function parseUriList(text: string): string[] {
	const paths: string[] = [];
	for (const line of text.split(/\r?\n/)) {
		appendLocalPath(paths, line.trim());
	}
	return paths;
}

function appendLocalPath(paths: string[], line: string) {
	if (line === "" || line.startsWith("#")) {
		return;
	}
	const path = localPathOf(line);
	if (path === "") {
		return;
	}
	paths.push(path);
}

// A URI with an authority other than the local machine names a file this
// explorer cannot reach through the filesystem.
function localPathOf(uri: string): string {
	for (const prefix of ["file:///", "file://localhost/"]) {
		if (uri.startsWith(prefix)) {
			return decodePath(uri.slice(prefix.length - 1));
		}
	}
	return "";
}

function decodePath(encoded: string): string {
	try {
		return encoded.split("/").map(decodeURIComponent).join("/");
	} catch {
		return "";
	}
}
