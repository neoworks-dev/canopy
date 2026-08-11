<script lang="ts">
	import { onMount, tick } from "svelte";
	import { Button } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import CaretLeftIcon from "phosphor-svelte/lib/CaretLeftIcon";
	import CaretRightIcon from "phosphor-svelte/lib/CaretRightIcon";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import EyeIcon from "phosphor-svelte/lib/EyeIcon";
	import EyeSlashIcon from "phosphor-svelte/lib/EyeSlashIcon";
	import GitBranchIcon from "phosphor-svelte/lib/GitBranchIcon";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import {
		AddBookmark,
		AddTag,
		Bookmarks,
		CopyPathsToClipboard,
		Devices,
		ForgetTag,
		HomeDirectory,
		KnownTags,
		ListDirectory,
		LoadSettings,
		OpenPath,
		OpenWith,
		Places,
		ReadClipboardPaths,
		RecentEntries,
		RemoveBookmark,
		RemoveTag,
		SaveSettings,
		SetDefaultApplication,
		TagsOf,
		WatchDirectories,
	} from "../wailsjs/go/main/FileService";
	import { IndexStatus, Search } from "../wailsjs/go/main/IndexService";
	import { settings as storedSettings } from "../wailsjs/go/models";
	import type { bookmarks, filesystem, proto, recents } from "../wailsjs/go/models";
	import ConfirmDialog from "./lib/ConfirmDialog.svelte";
	import ConflictDialog from "./lib/ConflictDialog.svelte";
	import ContextMenu from "./lib/ContextMenu.svelte";
	import FilePane from "./lib/FilePane.svelte";
	import JumpField from "./lib/JumpField.svelte";
	import OpenWithDialog from "./lib/OpenWithDialog.svelte";
	import OperationProgress from "./lib/OperationProgress.svelte";
	import PaneResizer from "./lib/PaneResizer.svelte";
	import PathBar from "./lib/PathBar.svelte";
	import PlacesSidebar from "./lib/PlacesSidebar.svelte";
	import PreviewPane from "./lib/PreviewPane.svelte";
	import SearchField from "./lib/SearchField.svelte";
	import TabStrip from "./lib/TabStrip.svelte";
	import TagDialog from "./lib/TagDialog.svelte";
	import ViewMenu from "./lib/ViewMenu.svelte";
	import {
		backgroundMenuIds,
		defaultKeymap,
		movementCommands,
		selectionMenuIds,
		staticCommandDescriptors,
	} from "./lib/builtinCommands";
	import { iconForCommand } from "./lib/commandIcons";
	import { CommandRegistry, type Command, type MenuItem } from "./lib/commands";
	import { chordFromEvent } from "./lib/keys";
	import {
		compileKeymap,
		resolveChord,
		shortcutFor,
		type ChordResolution,
		type Keybinding,
	} from "./lib/keymap";
	import type { WhenContext } from "./lib/whenClause";
	import { defaultColumnIds, type ColumnId } from "./lib/columns";
	import { describeError } from "./lib/errors";
	import { parentDirectoryOf } from "./lib/format";
	import {
		canGoBack,
		canGoForward,
		currentLocation,
		historyAfterBack,
		historyAfterForward,
		type History,
	} from "./lib/history";
	import type { Movement } from "./lib/movement";
	import { emptyPaneStatus, type PaneStatus } from "./lib/paneStatus";
	import { defaultPaneWidths, paneGutterWidth } from "./lib/panes";
	import { sectionsAfterToggle, type SidebarSectionId } from "./lib/sidebar";
	import { itemCount, summarize, type StatusInput } from "./lib/statusStrip";
	import { defaultViewSettings, viewSettingsFrom, type ViewMode } from "./lib/settings";
	import {
		defaultSortOrder,
		sortOrderAfterHeaderClick,
		sortOrderReversed,
		sortOrderWithDirectoriesFirst,
		type SortKey,
		type SortOrder,
	} from "./lib/sorting";
	import { OperationQueue } from "./lib/operations.svelte";
	import {
		activateNeighbouringTab,
		activateTab,
		activeGroup,
		activeTab,
		closeGroup,
		closeTab,
		directoryLocation,
		focusOtherGroup,
		initialLayout,
		locationFromKey,
		maximumGroups,
		openTab,
		splitGroup,
		watchedPaths,
		withActiveGroup,
		withHistory,
		withLocation,
		type Layout,
		type TabLocation,
	} from "./lib/tabs";

	// Long enough that typing a word does not fire a query per keystroke, short
	// enough that results feel immediate.
	const searchDebounceMilliseconds = 150;
	// Dragging a gutter changes a setting on every pointer move; the file is
	// written once the drag settles instead of once per frame.
	const settingsSaveDebounceMilliseconds = 300;
	const statusPollMilliseconds = 3000;

	type SearchOutcome = {
		hits: proto.Hit[];
		truncated: boolean;
		errorMessage: string;
	};

	// The menu holds only where it was opened and which set it shows: what each
	// row does depends on the selection, which the pane reports a moment after
	// the click that opened it.
	type MenuState = {
		x: number;
		y: number;
		kind: "selection" | "background";
	};

	let latestPaneId = 0;
	let homePath = $state("");
	let layout = $state<Layout>(initialLayout("", nextId));
	let statuses = $state<Record<string, PaneStatus>>({});

	// Panes are driven imperatively for the actions that belong to whatever they
	// have selected, so the command that runs reaches the pane in focus.
	const panes: Record<string, FilePane> = {};

	let places = $state<filesystem.Place[]>([]);
	let devices = $state<filesystem.Device[]>([]);
	let pinned = $state<bookmarks.Bookmark[]>([]);
	let recent = $state<recents.Entry[]>([]);
	let tagNames = $state<string[]>([]);
	let collapsedSections = $state<SidebarSectionId[]>([]);
	// The tags on the selection, read when the dialog opens so it can show them.
	let selectionTags = $state<string[]>([]);
	let isTagDialogOpen = $state(false);

	let showHidden = $state(false);
	let viewMode = $state<ViewMode>("rows");
	let visibleColumnIds = $state<ColumnId[]>(defaultColumnIds);
	let sortOrder = $state<SortOrder>(defaultSortOrder);
	// Nothing is written back until the stored settings have been applied, or the
	// first save would overwrite them with defaults.
	let areSettingsLoaded = $state(false);
	let isEditingPath = $state(false);
	let searchInput = $state<HTMLInputElement>();
	let sidebarWidth = $state(defaultPaneWidths.sidebar);
	let detailsWidth = $state(defaultPaneWidths.details);
	let alwaysShowDetails = $state(false);

	let clipboardPaths = $state<string[]>([]);
	let isClipboardCut = $state(false);

	let menu = $state<MenuState | null>(null);
	let isConfirmingDelete = $state(false);
	let openWithPath = $state("");
	let isJumpOpen = $state(false);
	// A failure that belongs to no pane: launching a file, tagging a selection.
	let actionError = $state("");
	// Chords typed so far in a sequence such as "g g", waiting for the rest.
	let pendingChords = $state<string[]>([]);
	let userKeybindings = $state<Keybinding[]>([]);

	let searchQuery = $state("");
	// Whether the location field is showing search rather than the path.
	let isSearchOpen = $state(false);
	// The directory a search is confined to, or "" for every indexed root.
	let searchScope = $state("");
	let searchHits = $state<proto.Hit[]>([]);
	let searchTruncated = $state(false);
	let searchErrorMessage = $state("");
	let isSearching = $state(false);
	let indexStatus = $state<proto.StatusResult | null>(null);

	// Guards against an early slow search — the first one also starts the daemon —
	// overwriting the results of a later, faster one.
	let latestSearchSequence = 0;

	const operations = new OperationQueue(() => reloadPanes());
	const registry = new CommandRegistry();
	registry.registerAll(builtinCommandList());

	const group = $derived(activeGroup(layout));
	const tab = $derived(activeTab(group));
	const status = $derived(statusOf(group.id));
	const currentPath = $derived(status.path);
	const selectedEntries = $derived(status.selected);
	const isSearchActive = $derived(searchQuery.trim().length > 0);
	const isIndexing = $derived(indexStatus?.state === "scanning");
	const commandContext = $derived(currentCommandContext());
	// The user's bindings come last, so any of them overrides a built-in.
	const keymap = $derived(compileKeymap([...defaultKeymap, ...userKeybindings]));
	const conflict = $derived(operations.conflict);
	// Only a cut marks entries as pending; a copy leaves them as they are.
	const cutPaths = $derived(new Set(pathsHeldByCut()));
	const isFolderPinned = $derived(pinned.some((bookmark) => bookmark.path === currentPath));
	// The pane describes one entry, so it earns its width when there is one to
	// describe — or when the user has asked for it to stay.
	const isDetailsOpen = $derived(detailsPaneBelongsOnScreen());
	const isSidebarOpen = $derived(places.length > 0);
	const strip = $derived(summarize(statusInput()));

	onMount(() => {
		start();
		loadSidebar();
		return operations.listen();
	});

	// Every pane on screen is followed, so a split keeps both halves current.
	$effect(() => {
		WatchDirectories(watchedPaths(layout));
	});

	// A tab that moves is a folder that was opened, which the sidebar lists.
	$effect(() => {
		tab.location;
		refreshRecents();
	});

	$effect(() => {
		const next = currentViewSettings();
		if (!areSettingsLoaded) {
			return;
		}
		const timer = setTimeout(() => SaveSettings(next), settingsSaveDebounceMilliseconds);
		return () => clearTimeout(timer);
	});

	$effect(() => {
		const query = searchQuery.trim();
		const scope = searchScope;
		if (!query) {
			clearSearch();
			return;
		}
		const timer = setTimeout(() => runSearch(query, scope), searchDebounceMilliseconds);
		return () => clearTimeout(timer);
	});

	// The index is built in the background, so a query typed during the first crawl
	// needs to say that the result set is still growing.
	$effect(() => {
		if (!isSearchActive) {
			return;
		}
		refreshIndexStatus();
		const timer = setInterval(refreshIndexStatus, statusPollMilliseconds);
		return () => clearInterval(timer);
	});

	// The stored settings decide where the window opens, so they are applied
	// before the first pane is built.
	async function start() {
		const restored = await restoreSettings();
		homePath = await resolveHome();
		layout = initialLayout(await openableDirectory(restored.lastDirectory), nextId);
	}

	async function resolveHome(): Promise<string> {
		try {
			return await HomeDirectory();
		} catch {
			return "/";
		}
	}

	// A stored directory that has since been deleted must not leave the window
	// sitting on an error card at launch.
	async function openableDirectory(stored: string): Promise<string> {
		if (!stored) {
			return homePath;
		}
		try {
			await ListDirectory(stored, sortOrder);
			return stored;
		} catch {
			return homePath;
		}
	}

	function nextId(): string {
		latestPaneId += 1;
		return `pane-${latestPaneId}`;
	}

	function statusOf(groupId: string): PaneStatus {
		const reported = statuses[groupId];
		if (!reported) {
			return emptyPaneStatus;
		}
		return reported;
	}

	function reportStatus(groupId: string, reported: PaneStatus) {
		statuses = { ...statuses, [groupId]: reported };
	}

	function paneFor(groupId: string): FilePane | undefined {
		return panes[groupId];
	}

	function activePane(): FilePane | undefined {
		return paneFor(group.id);
	}

	/** Re-reads every pane, since an operation can touch any of them. */
	function reloadPanes() {
		layout.groups.forEach((paneGroup) => paneFor(paneGroup.id)?.reload());
	}

	async function restoreSettings() {
		const stored = await loadStoredSettings();
		viewMode = stored.viewMode;
		visibleColumnIds = stored.columns;
		showHidden = stored.showHidden;
		sortOrder = stored.sort;
		sidebarWidth = stored.panes.sidebar;
		detailsWidth = stored.panes.details;
		collapsedSections = stored.collapsedSections;
		alwaysShowDetails = stored.alwaysShowDetails;
		userKeybindings = stored.keybindings;
		areSettingsLoaded = true;
		return stored;
	}

	async function loadStoredSettings() {
		try {
			return viewSettingsFrom(await LoadSettings());
		} catch {
			// Unreadable settings are not worth refusing to start over.
			return defaultViewSettings;
		}
	}

	// The keymap is left out on purpose: nothing here authors one, and the store
	// puts the file's own back before every write.
	function currentViewSettings(): storedSettings.Settings {
		return storedSettings.Settings.createFrom({
			viewMode,
			columns: visibleColumnIds,
			showHidden,
			sort: sortOrder,
			lastDirectory: currentPath,
			panes: { sidebar: sidebarWidth, details: detailsWidth },
			collapsedSections,
			alwaysShowDetails,
		});
	}

	// The sidebar is a convenience, so a lookup that fails leaves its group empty
	// rather than blocking the listing behind an error.
	async function loadSidebar() {
		places = await orEmpty(Places());
		devices = await orEmpty(Devices());
		await refreshPins();
		await refreshRecents();
		await refreshTagNames();
	}

	async function refreshPins() {
		pinned = await orEmpty(Bookmarks());
	}

	async function refreshRecents() {
		recent = await orEmpty(RecentEntries());
	}

	async function refreshTagNames() {
		tagNames = await orEmpty(KnownTags());
	}

	async function orEmpty<Item>(request: Promise<Item[]>): Promise<Item[]> {
		try {
			return await request;
		} catch {
			return [];
		}
	}

	function navigateTo(path: string) {
		showLocation(directoryLocation(path));
	}

	function showLocation(location: TabLocation) {
		isEditingPath = false;
		layout = withLocation(layout, location);
	}

	function openPlace(path: string) {
		closeSearch();
		navigateTo(path);
	}

	function openTrash() {
		closeSearch();
		showLocation({ kind: "trash" });
	}

	function openTagPane(name: string) {
		closeSearch();
		showLocation({ kind: "tag", name });
	}

	function goUp() {
		if (!status.parentPath) {
			return;
		}
		navigateTo(status.parentPath);
	}

	function goHome() {
		closeSearch();
		navigateTo(homePath);
	}

	function goBack() {
		stepThroughHistory(historyAfterBack(tab.history));
	}

	function goForward() {
		stepThroughHistory(historyAfterForward(tab.history));
	}

	// Loads what the history now points at without recording another visit.
	function stepThroughHistory(moved: History) {
		if (moved === tab.history) {
			return;
		}
		layout = withHistory(layout, locationFromKey(currentLocation(moved)), moved);
	}

	function openNewTab() {
		layout = openTab(layout, tab.location, nextId);
	}

	function closeActiveTab() {
		layout = closeTab(layout, group.id, tab.id);
	}

	function splitPane() {
		layout = splitGroup(layout, nextId);
	}

	function closeActivePane() {
		layout = closeGroup(layout, group.id);
	}

	function focusPaneBeside() {
		layout = focusOtherGroup(layout);
		focusActivePane();
	}

	async function focusActivePane() {
		await tick();
		activePane()?.focusList();
	}

	function activateGroup(groupId: string) {
		layout = withActiveGroup(layout, groupId);
	}

	// A recently opened folder is browsed; a recently opened file is handed back
	// to the application that opens it.
	function openRecent(entry: recents.Entry) {
		if (entry.isDir) {
			openPlace(entry.path);
			return;
		}
		launch(entry.path);
	}

	function toggleSection(section: SidebarSectionId) {
		collapsedSections = sectionsAfterToggle(collapsedSections, section);
	}

	async function pinCurrentFolder() {
		await AddBookmark(currentPath);
		await refreshPins();
	}

	async function unpinFolder(path: string) {
		await RemoveBookmark(path);
		await refreshPins();
	}

	async function forgetTag(name: string) {
		await ForgetTag(name);
		await refreshTagNames();
		closeTagPane(name);
	}

	// The pane of a tag that is no longer offered has nothing left to show.
	function closeTagPane(name: string) {
		if (tab.location.kind !== "tag" || tab.location.name !== name) {
			return;
		}
		navigateTo(currentPath || homePath);
	}

	async function launch(path: string) {
		actionError = "";
		try {
			await OpenPath(path);
		} catch (error) {
			actionError = describeError(error);
			return;
		}
		await refreshRecents();
	}

	async function runSearch(query: string, scope: string) {
		const sequence = ++latestSearchSequence;
		isSearching = true;
		const outcome = await requestSearch(query, scope);
		if (sequence !== latestSearchSequence) {
			return;
		}
		applySearchOutcome(outcome);
		isSearching = false;
	}

	async function requestSearch(query: string, scope: string): Promise<SearchOutcome> {
		try {
			const result = await Search(query, scope);
			return { hits: result.hits, truncated: result.truncated, errorMessage: "" };
		} catch (error) {
			return { hits: [], truncated: false, errorMessage: describeError(error) };
		}
	}

	function applySearchOutcome(outcome: SearchOutcome) {
		searchHits = outcome.hits;
		searchTruncated = outcome.truncated;
		searchErrorMessage = outcome.errorMessage;
	}

	// The scope outlives an empty query: opening the field on a folder and typing
	// into it must still search that folder.
	function clearSearch() {
		latestSearchSequence++;
		searchHits = [];
		searchTruncated = false;
		searchErrorMessage = "";
		isSearching = false;
	}

	async function refreshIndexStatus() {
		try {
			indexStatus = await IndexStatus();
		} catch {
			indexStatus = null;
		}
	}

	// A hit is opened where it lives: the folder holding it is listed and the entry
	// itself is focused, so the next keystroke acts on it.
	async function openHit(hit: proto.Hit) {
		closeSearch();
		navigateTo(parentDirectoryOf(hit.path));
		await tick();
		activePane()?.revealPath(hit.path);
	}

	// The scope is set before the field takes focus, so the first keystroke
	// already queries the right thing.
	async function openSearch(scope: string) {
		searchScope = scope;
		searchQuery = "";
		isSearchOpen = true;
		await tick();
		searchInput?.focus();
	}

	function searchThisFolder() {
		openSearch(currentPath);
	}

	function searchEverywhere() {
		openSearch("");
	}

	function closeSearch() {
		searchQuery = "";
		searchScope = "";
		isSearchOpen = false;
		activePane()?.focusList();
	}

	function pathsHeldByCut(): string[] {
		if (!isClipboardCut) {
			return [];
		}
		return clipboardPaths;
	}

	function currentCommandContext(): WhenContext {
		return {
			listActive: status.isListActive,
			searchOpen: isSearchOpen,
			trashOpen: tab.location.kind === "trash",
			jumpOpen: isJumpOpen,
			selectionCount: selectedEntries.length,
			clipboardCount: clipboardPaths.length,
			entryCount: status.entryCount,
			canUndo: operations.undoLabel !== "",
			canGoBack: canGoBack(tab.history),
			canGoForward: canGoForward(tab.history),
			canGoUp: status.parentPath !== "",
			viewMode,
			folderPinned: isFolderPinned,
			tabCount: group.tabs.length,
			paneCount: layout.groups.length,
			canSplit: layout.groups.length < maximumGroups,
		};
	}

	function detailsPaneBelongsOnScreen(): boolean {
		if (alwaysShowDetails) {
			return true;
		}
		return selectedEntries.length === 1;
	}

	// The strip carries what the details pane would have said, so it only has
	// something to report while a listing is the pane in focus.
	function statusInput(): StatusInput {
		if (!status.isListActive) {
			return { entryCount: 0, hiddenCount: 0, selected: [] };
		}
		return {
			entryCount: status.entryCount,
			hiddenCount: status.hiddenCount,
			selected: selectedEntries,
		};
	}

	// The strip lines up with the listing rather than the window, so the space a
	// flanking pane takes is left empty beside it.
	function stripInset(): string {
		return `margin-left: ${insetFor(isSidebarOpen, sidebarWidth)}px; margin-right: ${insetFor(isDetailsOpen, detailsWidth)}px`;
	}

	// The gutter between the panes is as wide as the resizer that sits in it.
	function insetFor(isOpen: boolean, width: number): number {
		if (!isOpen) {
			return 0;
		}
		return width + paneGutterWidth;
	}

	function selectionMenuGroups(): MenuItem[][] {
		return menuGroups(selectionMenuIds);
	}

	function backgroundMenuGroups(): MenuItem[][] {
		return menuGroups(backgroundMenuIds);
	}

	function menuGroups(menuLayout: string[][]): MenuItem[][] {
		return menuLayout.map((ids) => ids.flatMap(menuItemFor));
	}

	// A command that is no longer registered drops out of the menu rather than
	// rendering a row that cannot run.
	function menuItemFor(id: string): MenuItem[] {
		const command = registry.byId(id);
		const icon = iconForCommand(id);
		if (!command || !icon) {
			return [];
		}
		return [
			{
				id,
				title: command.title,
				icon,
				enabled: registry.isEnabled(id, commandContext),
				shortcut: shortcutFor(keymap, id, commandContext),
			},
		];
	}

	function openSelectionMenu(event: MouseEvent) {
		menu = { x: event.clientX, y: event.clientY, kind: "selection" };
	}

	function openBackgroundMenu(event: MouseEvent) {
		menu = { x: event.clientX, y: event.clientY, kind: "background" };
	}

	function menuGroupsFor(kind: MenuState["kind"]): MenuItem[][] {
		if (kind === "selection") {
			return selectionMenuGroups();
		}
		return backgroundMenuGroups();
	}

	function closeMenu() {
		menu = null;
	}

	function runCommand(id: string) {
		closeMenu();
		registry.run(id, commandContext);
	}

	function toggleHidden() {
		showHidden = !showHidden;
	}

	function setColumns(ids: ColumnId[]) {
		visibleColumnIds = ids;
	}

	// The Go side sorts, so a new order means re-reading the directories rather
	// than reordering what is on screen.
	function sortByColumn(key: SortKey) {
		sortOrder = sortOrderAfterHeaderClick(sortOrder, key);
	}

	function setDirectoriesFirst(directoriesFirst: boolean) {
		sortOrder = sortOrderWithDirectoriesFirst(sortOrder, directoriesFirst);
	}

	function reverseSort() {
		sortOrder = sortOrderReversed(sortOrder);
	}

	function setViewMode(mode: ViewMode) {
		viewMode = mode;
	}

	function commandHandlers(): Record<string, () => void> {
		return {
			"navigate.open": () => activePane()?.openSelection(),
			"navigate.openWith": startOpenWith,
			"navigate.parent": goUp,
			"navigate.back": goBack,
			"navigate.forward": goForward,
			"navigate.home": goHome,
			"navigate.trash": openTrash,
			"navigate.editPath": () => (isEditingPath = true),

			"list.selectAll": () => activePane()?.selectEverything(),
			"list.clearSelection": () => activePane()?.clearSelection(),
			"list.toggleSelection": () => activePane()?.toggleFocusedEntry(),
			"list.find": () => activePane()?.openFind(),
			"zoxide.jump": () => (isJumpOpen = true),

			"tab.new": openNewTab,
			"tab.close": closeActiveTab,
			"tab.next": () => (layout = activateNeighbouringTab(layout, 1)),
			"tab.previous": () => (layout = activateNeighbouringTab(layout, -1)),
			"pane.split": splitPane,
			"pane.close": closeActivePane,
			"pane.focusOther": focusPaneBeside,

			"view.rows": () => setViewMode("rows"),
			"view.grid": () => setViewMode("grid"),
			"view.toggleHidden": toggleHidden,
			"view.sortByName": () => sortByColumn("name"),
			"view.sortBySize": () => sortByColumn("size"),
			"view.sortByModified": () => sortByColumn("modified"),
			"view.sortByPermissions": () => sortByColumn("permissions"),
			"view.reverseSort": reverseSort,
			"view.toggleDirectoriesFirst": () => setDirectoriesFirst(!sortOrder.directoriesFirst),

			"file.cut": cutSelection,
			"file.copy": copySelection,
			"file.paste": pasteClipboard,
			"file.rename": () => activePane()?.startRename(),
			"file.trash": trashSelection,
			"file.deletePermanently": () => (isConfirmingDelete = true),
			"file.newFolder": () => activePane()?.createEntry("folder"),
			"file.newFile": () => activePane()?.createEntry("file"),
			"file.undo": () => operations.undo(),

			"tags.edit": startTagging,
			"bookmark.pin": pinCurrentFolder,
			"bookmark.unpin": () => unpinFolder(currentPath),

			"search.here": searchThisFolder,
			"search.global": searchEverywhere,
			"search.close": closeSearch,
		};
	}

	function builtinCommandList(): Command[] {
		const handlers = commandHandlers();
		const described = staticCommandDescriptors.map((descriptor) => ({
			...descriptor,
			icon: iconForCommand(descriptor.id),
			run: handlerFor(handlers, descriptor.id),
		}));
		return [...described, ...movementCommandList()];
	}

	function movementCommandList(): Command[] {
		return movementCommands.map((command) => ({
			id: command.id,
			title: command.title,
			category: "Listing",
			when: "listActive",
			run: () => moveSelection(command.movement, command.extend),
		}));
	}

	function moveSelection(movement: Movement, extend: boolean) {
		activePane()?.moveSelection(movement, extend);
	}

	// A built-in without a handler is a wiring mistake, and one that would show up
	// as a shortcut that quietly does nothing.
	function handlerFor(handlers: Record<string, () => void>, id: string): () => void {
		const handler = handlers[id];
		if (!handler) {
			throw new Error(`command ${id} has no handler`);
		}
		return handler;
	}

	function startOpenWith() {
		if (selectedEntries.length !== 1) {
			return;
		}
		openWithPath = selectedEntries[0].path;
	}

	async function openWithApplication(applicationID: string, remember: boolean) {
		const path = openWithPath;
		openWithPath = "";
		try {
			await OpenWith(path, applicationID);
			await rememberApplication(path, applicationID, remember);
		} catch (error) {
			actionError = describeError(error);
		}
	}

	async function rememberApplication(path: string, applicationID: string, remember: boolean) {
		if (!remember) {
			return;
		}
		await SetDefaultApplication(path, applicationID);
	}

	function copySelection() {
		clipboardPaths = selectedPathList();
		isClipboardCut = false;
		publishClipboard();
	}

	function cutSelection() {
		clipboardPaths = selectedPathList();
		isClipboardCut = true;
		publishClipboard();
	}

	// Publishing the uri-list lets other applications paste the selection; a
	// missing helper binary only costs interoperability, not the internal copy.
	async function publishClipboard() {
		try {
			await CopyPathsToClipboard(clipboardPaths);
		} catch {
			return;
		}
	}

	async function pasteClipboard() {
		const sources = await pastableSources();
		if (sources.length === 0) {
			return;
		}
		await operations.start(pasteKind(), sources, currentPath);
		clearClipboardAfterMove();
	}

	async function pastableSources(): Promise<string[]> {
		if (clipboardPaths.length > 0) {
			return clipboardPaths;
		}
		try {
			return await ReadClipboardPaths();
		} catch {
			return [];
		}
	}

	function pasteKind(): "copy" | "move" {
		if (isClipboardCut) {
			return "move";
		}
		return "copy";
	}

	function clearClipboardAfterMove() {
		if (!isClipboardCut) {
			return;
		}
		clipboardPaths = [];
		isClipboardCut = false;
	}

	function trashSelection() {
		operations.start("trash", selectedPathList(), "");
	}

	function confirmPermanentDelete() {
		isConfirmingDelete = false;
		operations.start("delete", selectedPathList(), "");
	}

	function selectedPathList(): string[] {
		return selectedEntries.map((entry) => entry.path);
	}

	// The dialog shows what the selection already carries, which only means
	// something when it is one file.
	async function startTagging() {
		selectionTags = await tagsOfSelection();
		isTagDialogOpen = true;
	}

	async function tagsOfSelection(): Promise<string[]> {
		if (selectedEntries.length !== 1) {
			return [];
		}
		try {
			return await TagsOf(selectedEntries[0].path);
		} catch {
			return [];
		}
	}

	async function applyTag(name: string) {
		actionError = "";
		try {
			await AddTag(selectedPathList(), name);
		} catch (error) {
			actionError = describeError(error);
		}
		await afterTagging();
	}

	async function dropTag(name: string) {
		actionError = "";
		try {
			await RemoveTag(selectedPathList(), name);
		} catch (error) {
			actionError = describeError(error);
		}
		await afterTagging();
	}

	async function afterTagging() {
		selectionTags = await tagsOfSelection();
		await refreshTagNames();
	}

	function jumpToFolder(path: string) {
		isJumpOpen = false;
		closeSearch();
		navigateTo(path);
	}

	/**
	 * One key handler for the whole window. Which surface a chord belongs to is a
	 * "when" clause on the binding rather than a listener on that surface, so a
	 * contributed binding can reach anywhere without needing its own listener.
	 *
	 * Typing into a field suspends the keymap entirely: that is what makes a bare
	 * letter safe to bind to a command.
	 */
	function handleWindowKeydown(event: KeyboardEvent) {
		if (isTypingTarget(event.target) || ownsTheKeyboard()) {
			pendingChords = [];
			return;
		}
		const chord = chordFromEvent(event);
		if (chord === "" || activatesFocusedControl(event.target, chord)) {
			return;
		}
		applyChord(chord, event);
	}

	// A focused button answers to space and enter itself. Running a command on the
	// same press would fire both.
	function activatesFocusedControl(target: EventTarget | null, chord: string): boolean {
		if (chord !== "space" && chord !== "enter") {
			return false;
		}
		if (!(target instanceof HTMLElement)) {
			return false;
		}
		return ["BUTTON", "A", "SELECT"].includes(target.tagName);
	}

	function isTypingTarget(target: EventTarget | null): boolean {
		if (!(target instanceof HTMLElement)) {
			return false;
		}
		return target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;
	}

	/** Surfaces that take the keyboard for themselves while they are open. */
	function ownsTheKeyboard(): boolean {
		if (Boolean(menu) || isConfirmingDelete || isTagDialogOpen) {
			return true;
		}
		return openWithPath !== "" || Boolean(conflict);
	}

	function applyChord(chord: string, event: KeyboardEvent) {
		const resolution = resolveChord(keymap, pendingChords, chord, commandContext);
		if (resolution.status === "pending") {
			pendingChords = [...pendingChords, chord];
			event.preventDefault();
			return;
		}
		pendingChords = [];
		runResolvedChord(resolution, event);
	}

	function runResolvedChord(resolution: ChordResolution, event: KeyboardEvent) {
		if (resolution.status !== "run") {
			return;
		}
		if (registry.run(resolution.command, commandContext)) {
			event.preventDefault();
		}
	}

	function searchPlaceholder(): string {
		if (!searchScope) {
			return "Search everywhere";
		}
		return "Search this folder";
	}

	function scopeLabel(): string {
		if (!searchScope) {
			return "";
		}
		return nameOfDirectory(searchScope);
	}

	function nameOfDirectory(path: string): string {
		const name = path.split("/").filter(Boolean).pop();
		if (!name) {
			return "/";
		}
		return name;
	}

	function hiddenToggleIcon(): IconComponent {
		if (showHidden) {
			return EyeIcon;
		}
		return EyeSlashIcon;
	}

	function indexingLabel(reported: proto.StatusResult): string {
		return `Indexing ${reported.filesIndexed.toLocaleString()} entries so far`;
	}

	// The result set is still growing while the first crawl runs, which is worth
	// saying next to a thin set of matches.
	function indexingNotice(): string {
		if (!isIndexing || !indexStatus) {
			return "";
		}
		return indexingLabel(indexStatus);
	}

	function taggingTitle(): string {
		if (selectedEntries.length === 1) {
			return `Tags for ${selectedEntries[0].name}`;
		}
		return `Tags for ${itemCount(selectedEntries.length)}`;
	}

	function deleteMessage(): string {
		return `${itemCount(selectedEntries.length)} will be deleted permanently. This cannot be undone.`;
	}

	function handleAuxClick(event: MouseEvent) {
		if (event.button === 3) {
			goBack();
		}
		if (event.button === 4) {
			goForward();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} onauxclick={handleAuxClick} />

<div class="flex h-full flex-col gap-2 bg-canvas p-2 select-none">
	<div class="flex shrink-0 items-center gap-2 px-1">
		<Button
			size="sm"
			variant="ghost"
			icon={CaretLeftIcon}
			disabled={!canGoBack(tab.history)}
			onclick={goBack}
		/>
		<Button
			size="sm"
			variant="ghost"
			icon={CaretRightIcon}
			disabled={!canGoForward(tab.history)}
			onclick={goForward}
		/>
		<Button size="sm" variant="ghost" icon={HouseIcon} onclick={goHome} />

		{#if isSearchOpen}
			<SearchField
				bind:value={searchQuery}
				bind:input={searchInput}
				placeholder={searchPlaceholder()}
				scope={scopeLabel()}
				hits={searchHits}
				truncated={searchTruncated}
				errorMessage={searchErrorMessage}
				indexingLabel={indexingNotice()}
				{isSearching}
				onOpenHit={openHit}
				onClose={closeSearch}
			/>
		{:else}
			<PathBar
				path={currentPath}
				isEditing={isEditingPath}
				onNavigate={navigateTo}
				onEditingChange={(editing) => (isEditingPath = editing)}
				onSearch={searchThisFolder}
			/>
		{/if}

		<ViewMenu
			{viewMode}
			onViewModeChange={setViewMode}
			visibleIds={visibleColumnIds}
			onChange={setColumns}
			directoriesFirst={sortOrder.directoriesFirst}
			onDirectoriesFirstChange={setDirectoriesFirst}
			{alwaysShowDetails}
			onAlwaysShowDetailsChange={(always) => (alwaysShowDetails = always)}
		/>

		<Button size="sm" variant="ghost" icon={hiddenToggleIcon()} onclick={toggleHidden} />
	</div>

	<div class="flex min-h-0 flex-1">
		{#if isSidebarOpen}
			<PlacesSidebar
				{places}
				{devices}
				{pinned}
				{recent}
				{tagNames}
				{currentPath}
				isTrashOpen={tab.location.kind === "trash"}
				openTag={tab.location.kind === "tag" ? tab.location.name : ""}
				{collapsedSections}
				width={sidebarWidth}
				onToggleSection={toggleSection}
				onNavigate={openPlace}
				onOpenRecent={openRecent}
				onOpenTrash={openTrash}
				onOpenTag={openTagPane}
				onRemoveBookmark={unpinFolder}
				onForgetTag={forgetTag}
			/>
			<PaneResizer
				side="left"
				width={sidebarWidth}
				label="Resize the places sidebar"
				onWidth={(width) => (sidebarWidth = width)}
			/>
		{/if}

		{#each layout.groups as paneGroup, index (paneGroup.id)}
			{#if index > 0}
				<div class="w-2 shrink-0"></div>
			{/if}
			<div
				class="flex min-w-0 flex-1 flex-col gap-1 rounded-xl"
				class:ring-1={layout.groups.length > 1 && paneGroup.id === layout.activeGroupId}
				class:ring-line-faint={layout.groups.length > 1 && paneGroup.id === layout.activeGroupId}
			>
				<TabStrip
					group={paneGroup}
					isActive={paneGroup.id === layout.activeGroupId}
					canSplit={layout.groups.length < maximumGroups}
					onSelect={(tabId) => (layout = activateTab(layout, paneGroup.id, tabId))}
					onClose={(tabId) => (layout = closeTab(layout, paneGroup.id, tabId))}
					onOpen={() => {
						activateGroup(paneGroup.id);
						openNewTab();
					}}
					onSplit={splitPane}
				/>
				<FilePane
					bind:this={panes[paneGroup.id]}
					location={activeTab(paneGroup).location}
					{viewMode}
					{visibleColumnIds}
					{sortOrder}
					{showHidden}
					{cutPaths}
					onStatus={(reported) => reportStatus(paneGroup.id, reported)}
					onNavigate={(location) => {
						activateGroup(paneGroup.id);
						showLocation(location);
					}}
					onActivate={() => activateGroup(paneGroup.id)}
					onLaunch={launch}
					onSelectionMenu={openSelectionMenu}
					onBackgroundMenu={openBackgroundMenu}
					onSort={sortByColumn}
					onChanged={() => operations.refreshUndoLabel()}
				/>
			</div>
		{/each}

		{#if isDetailsOpen}
			<PaneResizer
				side="right"
				width={detailsWidth}
				label="Resize the details pane"
				onWidth={(width) => (detailsWidth = width)}
			/>

			<PreviewPane entries={selectedEntries} width={detailsWidth} />
		{/if}
	</div>

	<!-- Status strip: floats on the canvas like the top bar, and lines up with
	     the listing rather than the window. -->
	<div class="flex h-8 shrink-0 items-center gap-2 px-1" style={stripInset()}>
		{#if status.branch}
			<span class="flex shrink-0 items-center gap-1 text-xs text-dim" title="On branch {status.branch}">
				<GitBranchIcon size={12} />
				<span class="max-w-40 truncate">{status.branch}</span>
			</span>
			<span class="shrink-0 text-faint">·</span>
		{/if}
		{#if actionError}
			<button
				type="button"
				onclick={() => (actionError = "")}
				class="flex min-w-0 items-center gap-2 rounded-md px-1 text-left"
			>
				<WarningIcon size={14} class="shrink-0 text-red" />
				<span class="min-w-0 truncate text-xs text-dim">{actionError}</span>
			</button>
		{:else}
			{#if strip.primary}
				<span class="min-w-0 truncate text-xs text-muted">{strip.primary}</span>
			{/if}
			{#if strip.secondary}
				<span class="shrink-0 text-xs text-faint">{strip.secondary}</span>
			{/if}
		{/if}
	</div>
</div>

{#if isJumpOpen}
	<JumpField {currentPath} onJump={jumpToFolder} onClose={() => (isJumpOpen = false)} />
{/if}

{#if menu}
	<ContextMenu
		x={menu.x}
		y={menu.y}
		groups={menuGroupsFor(menu.kind)}
		onSelect={runCommand}
		onClose={closeMenu}
	/>
{/if}

{#if conflict}
	<ConflictDialog
		prompt={conflict}
		onResolve={(resolution, applyToAll) => operations.resolve(resolution, applyToAll)}
	/>
{/if}

{#if isConfirmingDelete}
	<ConfirmDialog
		title="Delete permanently?"
		message={deleteMessage()}
		confirmLabel="Delete"
		onConfirm={confirmPermanentDelete}
		onCancel={() => (isConfirmingDelete = false)}
	/>
{/if}

{#if isTagDialogOpen}
	<TagDialog
		title={taggingTitle()}
		known={tagNames}
		applied={selectionTags}
		onApply={applyTag}
		onRemove={dropTag}
		onClose={() => (isTagDialogOpen = false)}
	/>
{/if}

{#if openWithPath}
	<OpenWithDialog
		path={openWithPath}
		onOpen={openWithApplication}
		onCancel={() => (openWithPath = "")}
	/>
{/if}

<OperationProgress
	operations={operations.operations}
	onPause={(id) => operations.pause(id)}
	onResume={(id) => operations.resume(id)}
	onCancel={(id) => operations.cancel(id)}
	onDismiss={(id) => operations.dismiss(id)}
/>
