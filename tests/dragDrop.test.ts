import { describe, expect, test } from "bun:test";
import {
	dragSourceOf,
	dropDestination,
	dropOperation,
	internalDragType,
	isAcceptableDrop,
	parseUriList,
	pathsForDrag,
	readDraggedPaths,
	toUriList,
	uriListType,
	type DragPayload,
} from "../frontend/src/lib/dragDrop";

const noModifiers = { ctrlKey: false, shiftKey: false, metaKey: false };

function payload(values: Record<string, string>): DragPayload {
	return {
		types: Object.keys(values),
		getData: (type: string) => {
			const value = values[type];
			if (value === undefined) {
				return "";
			}
			return value;
		},
	};
}

describe("what a drag carries", () => {
	test("dragging an entry inside the selection drags the whole selection", () => {
		expect(pathsForDrag(["/a/one", "/a/two"], "/a/two")).toEqual(["/a/one", "/a/two"]);
	});

	test("dragging an entry outside the selection drags only that entry", () => {
		expect(pathsForDrag(["/a/one", "/a/two"], "/a/three")).toEqual(["/a/three"]);
	});
});

describe("what dropping means", () => {
	test("a drag inside the explorer moves", () => {
		expect(dropOperation("internal", noModifiers)).toBe("move");
	});

	test("a drag from another application copies", () => {
		expect(dropOperation("external", noModifiers)).toBe("copy");
	});

	test("ctrl forces a copy", () => {
		expect(dropOperation("internal", { ...noModifiers, ctrlKey: true })).toBe("copy");
	});

	test("shift forces a move even for a drag from outside", () => {
		expect(dropOperation("external", { ...noModifiers, shiftKey: true })).toBe("move");
	});

	test("ctrl wins over shift, so a mistyped chord copies rather than moves", () => {
		expect(dropOperation("internal", { ...noModifiers, ctrlKey: true, shiftKey: true })).toBe(
			"copy",
		);
	});
});

describe("where a drop lands", () => {
	test("a folder under the pointer takes the drop", () => {
		expect(dropDestination({ path: "/a/folder", isDir: true }, "/a")).toBe("/a/folder");
	});

	test("a file under the pointer means the folder it sits in", () => {
		expect(dropDestination({ path: "/a/note.txt", isDir: false }, "/a")).toBe("/a");
	});

	test("empty space means the folder the pane is showing", () => {
		expect(dropDestination(null, "/a")).toBe("/a");
	});
});

describe("whether a drop is worth starting", () => {
	test("a move into another folder is", () => {
		expect(isAcceptableDrop(["/a/one"], "/b", "move")).toBe(true);
	});

	test("a drop carrying nothing is not", () => {
		expect(isAcceptableDrop([], "/b", "move")).toBe(false);
	});

	test("a drop with nowhere to land is not", () => {
		expect(isAcceptableDrop(["/a/one"], "", "move")).toBe(false);
	});

	test("a folder dropped on itself is not", () => {
		expect(isAcceptableDrop(["/a/folder"], "/a/folder", "move")).toBe(false);
	});

	test("a folder dropped inside its own subtree is not", () => {
		expect(isAcceptableDrop(["/a/folder"], "/a/folder/nested", "move")).toBe(false);
	});

	test("a folder dropped beside a sibling of a matching prefix still is", () => {
		expect(isAcceptableDrop(["/a/folder"], "/a/folder-two", "move")).toBe(true);
	});

	test("a move into the folder the entries already sit in is not", () => {
		expect(isAcceptableDrop(["/a/one", "/a/two"], "/a", "move")).toBe(false);
	});

	test("a move is worth starting when only some entries are already there", () => {
		expect(isAcceptableDrop(["/a/one", "/b/two"], "/a", "move")).toBe(true);
	});

	test("a copy into the folder the entries sit in is a real request", () => {
		expect(isAcceptableDrop(["/a/one"], "/a", "copy")).toBe(true);
	});
});

describe("reading a transfer", () => {
	test("a transfer carrying the explorer's own type came from inside", () => {
		expect(dragSourceOf(payload({ [internalDragType]: "[]" }))).toBe("internal");
	});

	test("a transfer carrying only a URI list came from outside", () => {
		expect(dragSourceOf(payload({ [uriListType]: "" }))).toBe("external");
	});

	test("the explorer's own paths are preferred over the URI list", () => {
		const transfer = payload({
			[internalDragType]: JSON.stringify(["/a/one"]),
			[uriListType]: "file:///b/two",
		});
		expect(readDraggedPaths(transfer)).toEqual(["/a/one"]);
	});

	test("a URI list is read when there is no payload of our own", () => {
		expect(readDraggedPaths(payload({ [uriListType]: "file:///b/two" }))).toEqual(["/b/two"]);
	});

	test("a payload that is not a list of paths reads as nothing", () => {
		expect(readDraggedPaths(payload({ [internalDragType]: "not json" }))).toEqual([]);
	});
});

describe("the URI list other applications read", () => {
	test("a path survives a round trip through it", () => {
		expect(parseUriList(toUriList(["/a/one", "/b/two"]))).toEqual(["/a/one", "/b/two"]);
	});

	test("a name with a space or a hash survives a round trip", () => {
		expect(parseUriList(toUriList(["/a/two words #1.txt"]))).toEqual(["/a/two words #1.txt"]);
	});

	test("comments and blank lines are skipped", () => {
		expect(parseUriList("# a comment\r\n\r\nfile:///a/one\r\n")).toEqual(["/a/one"]);
	});

	test("a URI naming another machine is left alone", () => {
		expect(parseUriList("file://elsewhere/a/one\r\nfile:///a/two")).toEqual(["/a/two"]);
	});

	test("a URI that is not a file is left alone", () => {
		expect(parseUriList("https://example.org/a\r\nfile:///a/two")).toEqual(["/a/two"]);
	});

	test("a localhost URI is a local path", () => {
		expect(parseUriList("file://localhost/a/one")).toEqual(["/a/one"]);
	});
});
