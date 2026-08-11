/**
 * The window's layout: one or more pane groups side by side, each with its own
 * tabs. Kept free of the DOM and of the filesystem, so every rule about what
 * closing a tab or splitting a group does can be checked on its own.
 *
 * A tab holds where it is and how it got there; the pane that renders it holds
 * nothing that has to survive a switch.
 */
import { emptyHistory, historyAfterVisit, type History } from "./history";

/** What a tab is showing. The listing is one of three panes, not the only one. */
export type TabLocation =
	| { kind: "directory"; path: string }
	| { kind: "trash" }
	| { kind: "tag"; name: string }
	/** A saved search, shown as a folder of whatever it matches now. */
	| { kind: "search"; name: string }
	/** The files under a folder that hold the same bytes as each other. */
	| { kind: "duplicates"; path: string }
	/** An archive, looked through one level at a time without unpacking it. */
	| { kind: "archive"; path: string; inside: string };

export type Tab = {
	id: string;
	location: TabLocation;
	history: History;
};

export type PaneGroup = {
	id: string;
	tabs: Tab[];
	activeTabId: string;
};

export type Layout = {
	groups: PaneGroup[];
	activeGroupId: string;
};

/** Two panes are as many as fit side by side without either becoming a column. */
export const maximumGroups = 2;

export function directoryLocation(path: string): TabLocation {
	return { kind: "directory", path };
}

export function initialLayout(path: string, nextId: () => string): Layout {
	const group = groupWith(newTab(directoryLocation(path), nextId), nextId);
	return { groups: [group], activeGroupId: group.id };
}

export function activeGroup(layout: Layout): PaneGroup {
	const group = layout.groups.find((candidate) => candidate.id === layout.activeGroupId);
	if (!group) {
		return layout.groups[0];
	}
	return group;
}

export function activeTab(group: PaneGroup): Tab {
	const tab = group.tabs.find((candidate) => candidate.id === group.activeTabId);
	if (!tab) {
		return group.tabs[0];
	}
	return tab;
}

/** The directories the panes are showing, for the watches that follow them. */
export function watchedPaths(layout: Layout): string[] {
	const paths = layout.groups.map((group) => directoryOf(activeTab(group)));
	return [...new Set(paths.filter((path) => path !== ""))];
}

export function directoryOf(tab: Tab): string {
	return directoryPathOf(tab.location);
}

/** The directory a location shows, empty for the panes that show none. */
export function directoryPathOf(location: TabLocation): string {
	if (location.kind !== "directory") {
		return "";
	}
	return location.path;
}

/** The name a tab is labelled with, which is the last part of where it is. */
export function tabTitle(tab: Tab): string {
	if (tab.location.kind === "trash") {
		return "Trash";
	}
	if (tab.location.kind === "tag" || tab.location.kind === "search") {
		return tab.location.name;
	}
	if (tab.location.kind === "duplicates") {
		return `Duplicates in ${folderName(tab.location.path)}`;
	}
	if (tab.location.kind === "archive") {
		return folderName(tab.location.path);
	}
	return folderName(tab.location.path);
}

function folderName(path: string): string {
	const name = path.split("/").filter(Boolean).pop();
	if (!name) {
		return "/";
	}
	return name;
}

export function withActiveGroup(layout: Layout, groupId: string): Layout {
	if (!layout.groups.some((group) => group.id === groupId)) {
		return layout;
	}
	return { ...layout, activeGroupId: groupId };
}

/** Moves the tab a group is showing, recording the visit in its history. */
export function withLocation(layout: Layout, location: TabLocation): Layout {
	return mapActiveTab(layout, (tab) => ({
		...tab,
		location,
		history: historyAfterVisit(tab.history, locationKey(location)),
	}));
}

/**
 * Moves a tab without recording a visit, which is what stepping through the
 * history it already holds is.
 */
export function withHistory(layout: Layout, location: TabLocation, history: History): Layout {
	return mapActiveTab(layout, (tab) => ({ ...tab, location, history }));
}

export function openTab(layout: Layout, location: TabLocation, nextId: () => string): Layout {
	const opened = newTab(location, nextId);
	return mapActiveGroup(layout, (group) => ({
		...group,
		tabs: [...group.tabs, opened],
		activeTabId: opened.id,
	}));
}

export function activateTab(layout: Layout, groupId: string, tabId: string): Layout {
	const focused = withActiveGroup(layout, groupId);
	return mapGroup(focused, groupId, (group) => ({ ...group, activeTabId: tabId }));
}

/**
 * Closing the last tab of a split group collapses the split; closing the last
 * tab of the last group is refused, since a window with no pane has nothing to
 * show.
 */
export function closeTab(layout: Layout, groupId: string, tabId: string): Layout {
	const group = layout.groups.find((candidate) => candidate.id === groupId);
	if (!group || !group.tabs.some((tab) => tab.id === tabId)) {
		return layout;
	}
	if (group.tabs.length === 1) {
		return closeGroup(layout, groupId);
	}
	return mapGroup(layout, groupId, (current) => withoutTab(current, tabId));
}

function withoutTab(group: PaneGroup, tabId: string): PaneGroup {
	const index = group.tabs.findIndex((tab) => tab.id === tabId);
	const remaining = group.tabs.filter((tab) => tab.id !== tabId);
	return { ...group, tabs: remaining, activeTabId: successorId(group, remaining, index, tabId) };
}

// Closing the tab being looked at moves to its neighbour on the right, or the
// one on the left when it was the last; closing any other leaves the active one
// where it is.
function successorId(
	group: PaneGroup,
	remaining: Tab[],
	index: number,
	tabId: string,
): string {
	if (group.activeTabId !== tabId) {
		return group.activeTabId;
	}
	return remaining[Math.min(index, remaining.length - 1)].id;
}

/** Splits the active group, giving the new one a tab on the same location. */
export function splitGroup(layout: Layout, nextId: () => string): Layout {
	if (layout.groups.length >= maximumGroups) {
		return layout;
	}
	const source = activeTab(activeGroup(layout));
	const group = groupWith(newTab(source.location, nextId), nextId);
	return { groups: [...layout.groups, group], activeGroupId: group.id };
}

/** Closing the only group is refused: the window would have no pane at all. */
export function closeGroup(layout: Layout, groupId: string): Layout {
	if (layout.groups.length === 1) {
		return layout;
	}
	const remaining = layout.groups.filter((group) => group.id !== groupId);
	return { groups: remaining, activeGroupId: remaining[0].id };
}

/** Moves to the group beside the active one, which is what a split is for. */
export function focusOtherGroup(layout: Layout): Layout {
	const index = layout.groups.findIndex((group) => group.id === layout.activeGroupId);
	const next = layout.groups[(index + 1) % layout.groups.length];
	return { ...layout, activeGroupId: next.id };
}

export function activateTabAt(layout: Layout, index: number): Layout {
	const group = activeGroup(layout);
	const tab = group.tabs[index];
	if (!tab) {
		return layout;
	}
	return activateTab(layout, group.id, tab.id);
}

/** Steps through the active group's tabs, wrapping at either end. */
export function activateNeighbouringTab(layout: Layout, step: number): Layout {
	const group = activeGroup(layout);
	const index = group.tabs.findIndex((tab) => tab.id === group.activeTabId);
	const count = group.tabs.length;
	return activateTab(layout, group.id, group.tabs[(index + step + count) % count].id);
}

function newTab(location: TabLocation, nextId: () => string): Tab {
	return {
		id: nextId(),
		location,
		history: historyAfterVisit(emptyHistory, locationKey(location)),
	};
}

function groupWith(tab: Tab, nextId: () => string): PaneGroup {
	return { id: nextId(), tabs: [tab], activeTabId: tab.id };
}

/**
 * History is a list of strings, and the panes that are not a directory have no
 * path of their own, so they are recorded under a name no directory can have.
 */
export function locationKey(location: TabLocation): string {
	if (location.kind === "trash") {
		return "trash:";
	}
	if (location.kind === "tag") {
		return `tag:${location.name}`;
	}
	if (location.kind === "search") {
		return `search:${location.name}`;
	}
	if (location.kind === "duplicates") {
		return `duplicates:${location.path}`;
	}
	// The level inside is part of where the tab is, so stepping back through the
	// history walks back out of the archive one folder at a time.
	if (location.kind === "archive") {
		return `archive:${location.path}!${location.inside}`;
	}
	return location.path;
}

export function locationFromKey(key: string): TabLocation {
	if (key === "trash:") {
		return { kind: "trash" };
	}
	if (key.startsWith("tag:")) {
		return { kind: "tag", name: key.slice("tag:".length) };
	}
	if (key.startsWith("search:")) {
		return { kind: "search", name: key.slice("search:".length) };
	}
	if (key.startsWith("duplicates:")) {
		return { kind: "duplicates", path: key.slice("duplicates:".length) };
	}
	if (key.startsWith("archive:")) {
		return archiveLocationFromKey(key.slice("archive:".length));
	}
	return directoryLocation(key);
}

// The archive's own path can hold anything but a newline, so the level inside is
// split off at the last separator rather than the first.
function archiveLocationFromKey(rest: string): TabLocation {
	const separator = rest.lastIndexOf("!");
	if (separator < 0) {
		return { kind: "archive", path: rest, inside: "" };
	}
	return { kind: "archive", path: rest.slice(0, separator), inside: rest.slice(separator + 1) };
}

function mapActiveGroup(layout: Layout, change: (group: PaneGroup) => PaneGroup): Layout {
	return mapGroup(layout, layout.activeGroupId, change);
}

function mapGroup(
	layout: Layout,
	groupId: string,
	change: (group: PaneGroup) => PaneGroup,
): Layout {
	return {
		...layout,
		groups: layout.groups.map((group) => changeMatching(group, groupId, change)),
	};
}

function changeMatching(
	group: PaneGroup,
	groupId: string,
	change: (group: PaneGroup) => PaneGroup,
): PaneGroup {
	if (group.id !== groupId) {
		return group;
	}
	return change(group);
}

function mapActiveTab(layout: Layout, change: (tab: Tab) => Tab): Layout {
	return mapActiveGroup(layout, (group) => ({
		...group,
		tabs: group.tabs.map((tab) => changeMatchingTab(tab, group.activeTabId, change)),
	}));
}

function changeMatchingTab(tab: Tab, tabId: string, change: (tab: Tab) => Tab): Tab {
	if (tab.id !== tabId) {
		return tab;
	}
	return change(tab);
}
