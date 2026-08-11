import { beforeEach, describe, expect, test } from "bun:test";
import { canGoBack, currentLocation } from "../frontend/src/lib/history";
import {
	activateNeighbouringTab,
	activateTab,
	activateTabAt,
	activeGroup,
	activeTab,
	closeGroup,
	closeTab,
	directoryLocation,
	focusOtherGroup,
	initialLayout,
	locationFromKey,
	locationKey,
	maximumGroups,
	openTab,
	splitGroup,
	tabTitle,
	watchedPaths,
	withHistory,
	withLocation,
	type Layout,
} from "../frontend/src/lib/tabs";

let counter = 0;
const nextId = () => `id-${++counter}`;

beforeEach(() => {
	counter = 0;
});

function layoutAt(path: string): Layout {
	return initialLayout(path, nextId);
}

describe("a fresh window", () => {
	test("opens on one tab in one pane", () => {
		const layout = layoutAt("/home/moritz");

		expect(layout.groups).toHaveLength(1);
		expect(activeGroup(layout).tabs).toHaveLength(1);
		expect(activeTab(activeGroup(layout)).location).toEqual(directoryLocation("/home/moritz"));
	});

	test("the first location is already in the tab's history", () => {
		const tab = activeTab(activeGroup(layoutAt("/home/moritz")));

		expect(currentLocation(tab.history)).toBe("/home/moritz");
		expect(canGoBack(tab.history)).toBe(false);
	});
});

describe("moving a tab", () => {
	test("records the visit in that tab's own history", () => {
		let layout = layoutAt("/home/moritz");

		layout = withLocation(layout, directoryLocation("/home/moritz/Notes"));

		const tab = activeTab(activeGroup(layout));
		expect(tab.location).toEqual(directoryLocation("/home/moritz/Notes"));
		expect(canGoBack(tab.history)).toBe(true);
	});

	test("leaves the other tabs where they are", () => {
		let layout = layoutAt("/home/moritz");
		layout = openTab(layout, directoryLocation("/tmp"), nextId);

		layout = withLocation(layout, directoryLocation("/var"));

		const group = activeGroup(layout);
		expect(group.tabs[0].location).toEqual(directoryLocation("/home/moritz"));
		expect(group.tabs[1].location).toEqual(directoryLocation("/var"));
	});

	test("stepping through history does not record another visit", () => {
		let layout = layoutAt("/a");
		layout = withLocation(layout, directoryLocation("/b"));
		const stepped = activeTab(activeGroup(layout)).history;

		layout = withHistory(layout, directoryLocation("/a"), { ...stepped, index: 0 });

		expect(activeTab(activeGroup(layout)).history.entries).toEqual(["/a", "/b"]);
	});
});

describe("opening and closing tabs", () => {
	test("a new tab opens beside the others and takes over", () => {
		let layout = layoutAt("/a");

		layout = openTab(layout, directoryLocation("/b"), nextId);

		const group = activeGroup(layout);
		expect(group.tabs).toHaveLength(2);
		expect(activeTab(group).location).toEqual(directoryLocation("/b"));
	});

	test("closing the tab in focus moves to the one on its right", () => {
		let layout = layoutAt("/a");
		layout = openTab(layout, directoryLocation("/b"), nextId);
		layout = openTab(layout, directoryLocation("/c"), nextId);
		const group = activeGroup(layout);
		layout = activateTab(layout, group.id, group.tabs[1].id);

		layout = closeTab(layout, group.id, group.tabs[1].id);

		expect(activeTab(activeGroup(layout)).location).toEqual(directoryLocation("/c"));
	});

	test("closing the last tab moves to the one on its left", () => {
		let layout = layoutAt("/a");
		layout = openTab(layout, directoryLocation("/b"), nextId);
		const group = activeGroup(layout);

		layout = closeTab(layout, group.id, group.tabs[1].id);

		expect(activeTab(activeGroup(layout)).location).toEqual(directoryLocation("/a"));
	});

	test("closing a tab that is not in focus leaves the focus alone", () => {
		let layout = layoutAt("/a");
		layout = openTab(layout, directoryLocation("/b"), nextId);
		const group = activeGroup(layout);

		layout = closeTab(layout, group.id, group.tabs[0].id);

		expect(activeTab(activeGroup(layout)).location).toEqual(directoryLocation("/b"));
	});

	test("the last tab of the only pane cannot be closed", () => {
		const layout = layoutAt("/a");
		const group = activeGroup(layout);

		expect(closeTab(layout, group.id, group.tabs[0].id)).toBe(layout);
	});

	test("stepping past the last tab wraps to the first", () => {
		let layout = layoutAt("/a");
		layout = openTab(layout, directoryLocation("/b"), nextId);

		layout = activateNeighbouringTab(layout, 1);

		expect(activeTab(activeGroup(layout)).location).toEqual(directoryLocation("/a"));
	});

	test("a tab can be picked by position, and a position with no tab is ignored", () => {
		let layout = layoutAt("/a");
		layout = openTab(layout, directoryLocation("/b"), nextId);

		expect(activeTab(activeGroup(activateTabAt(layout, 0))).location).toEqual(
			directoryLocation("/a"),
		);
		expect(activateTabAt(layout, 7)).toBe(layout);
	});
});

describe("splitting", () => {
	test("the new pane opens on the same location and takes the focus", () => {
		let layout = layoutAt("/a");

		layout = splitGroup(layout, nextId);

		expect(layout.groups).toHaveLength(2);
		expect(activeTab(activeGroup(layout)).location).toEqual(directoryLocation("/a"));
		expect(layout.activeGroupId).toBe(layout.groups[1].id);
	});

	test("the panes have separate tabs", () => {
		let layout = layoutAt("/a");
		layout = splitGroup(layout, nextId);

		layout = openTab(layout, directoryLocation("/b"), nextId);

		expect(layout.groups[0].tabs).toHaveLength(1);
		expect(layout.groups[1].tabs).toHaveLength(2);
	});

	test("splitting stops at the number of panes that fit", () => {
		let layout = layoutAt("/a");
		for (let index = 0; index < maximumGroups + 2; index++) {
			layout = splitGroup(layout, nextId);
		}

		expect(layout.groups).toHaveLength(maximumGroups);
	});

	test("closing the last tab of a split pane collapses the split", () => {
		let layout = layoutAt("/a");
		layout = splitGroup(layout, nextId);
		const group = activeGroup(layout);

		layout = closeTab(layout, group.id, group.tabs[0].id);

		expect(layout.groups).toHaveLength(1);
	});

	test("the only pane cannot be closed", () => {
		const layout = layoutAt("/a");

		expect(closeGroup(layout, layout.activeGroupId)).toBe(layout);
	});

	test("the focus moves between the panes", () => {
		let layout = layoutAt("/a");
		layout = splitGroup(layout, nextId);

		layout = focusOtherGroup(layout);

		expect(layout.activeGroupId).toBe(layout.groups[0].id);
	});
});

describe("what is watched and what is labelled", () => {
	test("each pane's directory is watched, and duplicates are watched once", () => {
		let layout = layoutAt("/a");
		layout = splitGroup(layout, nextId);

		expect(watchedPaths(layout)).toEqual(["/a"]);
	});

	test("a pane showing no directory contributes nothing to watch", () => {
		const layout = withLocation(layoutAt("/a"), { kind: "trash" });

		expect(watchedPaths(layout)).toEqual([]);
	});

	test("a tab is labelled with the last part of where it is", () => {
		const tab = activeTab(activeGroup(layoutAt("/home/moritz/Notes")));

		expect(tabTitle(tab)).toBe("Notes");
	});

	test("the root has a label of its own rather than an empty one", () => {
		expect(tabTitle(activeTab(activeGroup(layoutAt("/"))))).toBe("/");
	});

	test("the panes that are not directories are labelled by what they show", () => {
		const trash = withLocation(layoutAt("/a"), { kind: "trash" });
		const tagged = withLocation(layoutAt("/a"), { kind: "tag", name: "work" });

		expect(tabTitle(activeTab(activeGroup(trash)))).toBe("Trash");
		expect(tabTitle(activeTab(activeGroup(tagged)))).toBe("work");
	});
});

describe("recording a location in history", () => {
	test("every kind survives the round trip through a history entry", () => {
		const locations = [
			directoryLocation("/home/moritz"),
			{ kind: "trash" } as const,
			{ kind: "tag", name: "work" } as const,
		];

		for (const location of locations) {
			expect(locationFromKey(locationKey(location))).toEqual(location);
		}
	});

	test("a tag whose name looks like a path is still read back as a tag", () => {
		expect(locationFromKey(locationKey({ kind: "tag", name: "/etc" }))).toEqual({
			kind: "tag",
			name: "/etc",
		});
	});
});
