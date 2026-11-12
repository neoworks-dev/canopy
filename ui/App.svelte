<script lang="ts">
	import { onMount, tick } from "svelte";
	import { View, type PointerEvent } from "@neoworks/svelte-gpui/ui";
	import { clipboard, window as appWindow } from "@neoworks/svelte-gpui";
	import Button from "./components/design/Button.svelte";
	import { caretLeft, caretRight, terminalWindow, textT } from "./icons";
	import { colors, root, space } from "./theme";
	import {
		AddBookmark,
		AddTag,
		Bookmarks,
		CopyPathsToClipboard,
		CreateLinks,
		Devices,
		ExtractionFolder,
		ForgetSearch,
		ForgetTag,
		HomeDirectory,
		KnownTags,
		ListDirectory,
		LoadSettings,
		OpenPath,
		OpenTerminal,
		OpenWith,
		Places,
		ReadClipboardPaths,
		RecentEntries,
		RemoveBookmark,
		RemoveTag,
		RememberWindowSize,
		RenameSearch,
		RunShell,
		SaveSearch,
		SearchContents,
		SaveSettings,
		SavedSearches,
		SetDefaultApplication,
		TagsOf,
		WatchDirectories,
	} from "./backend/FileService";
	import { IndexStatus, Search } from "./backend/IndexService";
	import type { bookmarks, filesystem, proto, recents, settings as storedSettings } from "./backend/models";
	import CommandPalette from "./components/CommandPalette.svelte";
	import ConfirmDialog from "./components/ConfirmDialog.svelte";
	import ConflictDialog from "./components/ConflictDialog.svelte";
	import ContextMenu from "./components/ContextMenu.svelte";
	import CreateEntryDialog from "./components/CreateEntryDialog.svelte";
	import FilePane from "./components/FilePane.svelte";
	import HelpPanel from "./components/HelpPanel.svelte";
	import JumpField from "./components/JumpField.svelte";
	import LinesDialog from "./components/LinesDialog.svelte";
	import OpenWithDialog from "./components/OpenWithDialog.svelte";
	import OperationProgress from "./components/OperationProgress.svelte";
	import PaneResizer from "./components/PaneResizer.svelte";
	import PathBar from "./components/PathBar.svelte";
	import PlacesSidebar from "./components/PlacesSidebar.svelte";
	import PreviewPane from "./components/PreviewPane.svelte";
	import SaveSearchDialog from "./components/SaveSearchDialog.svelte";
	import SearchField from "./components/SearchField.svelte";
	import TabStrip from "./components/TabStrip.svelte";
	import SpotDialog from "./components/SpotDialog.svelte";
	import StatusBar from "./components/StatusBar.svelte";
	import TagDialog from "./components/TagDialog.svelte";
	import TaskManager from "./components/TaskManager.svelte";
	import WhichKey from "./components/WhichKey.svelte";
	import TextPrompt from "./components/TextPrompt.svelte";
	import ViewMenu from "./components/ViewMenu.svelte";
	import {
		backgroundMenuIds,
		copyTextCommands,
		defaultKeymap,
		linemodeCommands,
		movementCommands,
		selectionMenuIds,
		sortCommands,
		staticCommandDescriptors,
		tabSelectCommands,
		type SortCommand,
	} from "./lib/builtinCommands";
	import { isArchiveName } from "./lib/archive";
	import { entriesToCreate, renameBuffer, renamePlan } from "./lib/bulkEdit";
	import { iconForCommand } from "./lib/commandIcons";
	import { copyTextFor, type CopyTextKind } from "./lib/copyText";
	import { continuationRows, helpRows, type HelpRow } from "./lib/help";
	import { linkDestination } from "./lib/links";
	import { CommandRegistry, type Command, type MenuItem } from "./lib/commands";
	import { chordFromEvent, sequenceLabel } from "./lib/keys";
	import {
		compileKeymap,
		resolveChord,
		shortcutFor,
		type ChordResolution,
		type CompiledBinding,
		type Keybinding,
	} from "./lib/keymap";
	import { evaluateWhen, type WhenContext } from "./lib/whenClause";
	import { defaultColumnIds, type ColumnId } from "./lib/columns";
	import { acceptsInternalDrop, dropOperation, isAcceptableDrop, type DropOperation } from "./lib/dragDrop";
	import { heldModifiers } from "./lib/heldModifiers.svelte";
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
	import { defaultPaneWidths } from "./lib/panes";
	import {
		searchDescription,
		suggestedName,
		type SavedSearch,
	} from "./lib/savedSearches";
	import { sectionsAfterToggle, type SidebarSectionId } from "./lib/sidebar";
	import { itemCount, summarize, type StatusInput } from "./lib/statusStrip";
	import { defaultViewSettings, viewSettingsFrom, type ViewMode } from "./lib/settings";
	import {
		defaultSortOrder,
		sortOrderAfterHeaderClick,
		sortOrderBy,
		sortOrderReversed,
		sortOrderWithDirectoriesFirst,
		type SortKey,
		type SortOrder,
	} from "./lib/sorting";
	import { OperationQueue } from "./lib/operations.svelte";
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
		maximumGroups,
		openTab,
		splitGroup,
		swapActiveTab,
		watchedPaths,
		withActiveGroup,
		withActiveTabTitle,
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
	const headerHeight = 48;
	// Yazi's K and J seek five units of the preview at a time.
	const previewSeekLines = 5;

	type SearchOutcome = {
		hits: proto.Hit[];
		truncated: boolean;
		errorMessage: string;
	};

	// Naming a search, either the one being saved or one already in the sidebar.
	type SearchDialogState = {
		title: string;
		initialName: string;
		description: string;
		/** The search being renamed, or null while the current query is saved. */
		renaming: SavedSearch | null;
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
	// have selected, so the command that runs reaches the pane in focus. Bound
	// state rather than a plain object, since the bindings write into it as the
	// panes mount and are torn down.
	let panes = $state<Record<string, FilePane>>({});

	let places = $state<filesystem.Place[]>([]);
	let devices = $state<filesystem.Device[]>([]);
	let pinned = $state<bookmarks.Bookmark[]>([]);
	let recent = $state<recents.Entry[]>([]);
	let tagNames = $state<string[]>([]);
	let savedSearches = $state<SavedSearch[]>([]);
	let searchDialog = $state<SearchDialogState | null>(null);
	let collapsedSections = $state<SidebarSectionId[]>([]);
	// The tags on the selection, read when the dialog opens so it can show them.
	let selectionTags = $state<string[]>([]);
	let isTagDialogOpen = $state(false);
	let isCreateDialogOpen = $state(false);
	let isBulkCreateOpen = $state(false);
	// The paths a bulk rename was opened on, empty while no rename is under way.
	let bulkRenamePaths = $state<string[]>([]);
	let bulkRenameError = $state("");
	let isRenamingTab = $state(false);
	let isHelpOpen = $state(false);
	let isTaskManagerOpen = $state(false);
	let isSpotOpen = $state(false);
	// Which shell prompt is open: Yazi's `;` runs in the background, `:` in a terminal.
	let shellPrompt = $state<"" | "background" | "terminal">("");
	let previewPane = $state<PreviewPane>();

	let showHidden = $state(false);
	let viewMode = $state<ViewMode>("rows");
	let visibleColumnIds = $state<ColumnId[]>(defaultColumnIds);
	let sortOrder = $state<SortOrder>(defaultSortOrder);
	// Nothing is written back until the stored settings have been applied, or the
	// first save would overwrite them with defaults.
	let areSettingsLoaded = $state(false);
	let isEditingPath = $state(false);
	let searchField = $state<SearchField>();
	let sidebarWidth = $state(defaultPaneWidths.sidebar);
	let detailsWidth = $state(defaultPaneWidths.details);
	let showDetails = $state(defaultViewSettings.showDetails);

	let clipboardPaths = $state<string[]>([]);
	let isClipboardCut = $state(false);

	let menu = $state<MenuState | null>(null);
	let isConfirmingDelete = $state(false);
	let openWithPath = $state("");
	let isJumpOpen = $state(false);
	let paletteContext = $state<WhenContext | null>(null);
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
	// Names come from the index; contents from ripgrep over the scope, as Yazi's S.
	let searchMode = $state<"names" | "contents">("names");
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
	const copiedPaths = $derived(new Set(pathsHeldByCopy()));
	// The saved search whose folder is on screen, which is where the pane is
	// rather than something to be kept in step with it.
	const openSearchName = $derived(searchNameOf(tab.location));
	const isFolderPinned = $derived(pinned.some((bookmark) => bookmark.path === currentPath));
	// The pane describes one entry, so it earns its width when there is one to
	// describe — or when the user has asked for it to stay.
	const isDetailsOpen = $derived(showDetails);
	const isSidebarOpen = $derived(places.length > 0);
	const strip = $derived(summarize(statusInput()));
	const whichKeyRows = $derived(pendingContinuations());

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
	// The build drops a bare `tab.location;`, so the dependency is read in a condition.
	$effect(() => {
		if (tab.location) {
			refreshRecents();
		}
	});

	$effect(() => {
		const next = currentViewSettings();
		if (!areSettingsLoaded) {
			return;
		}
		const timer = setTimeout(() => SaveSettings(next), settingsSaveDebounceMilliseconds);
		return () => clearTimeout(timer);
	});

	// The window's size is kept for the next launch; where it sits is up to the
	// compositor.
	$effect(() => {
		const { width, height } = appWindow.size;
		const timer = setTimeout(() => RememberWindowSize(width, height), settingsSaveDebounceMilliseconds);
		return () => clearTimeout(timer);
	});

	$effect(() => {
		const query = searchQuery.trim();
		const scope = searchScope;
		const mode = searchMode;
		if (!query) {
			clearSearch();
			return;
		}
		const timer = setTimeout(() => runSearch(query, scope, mode), searchDebounceMilliseconds);
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
		showDetails = stored.showDetails;
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
	function currentViewSettings(): storedSettings.ViewSettings {
		return {
			viewMode,
			columns: visibleColumnIds,
			showHidden,
			sort: sortOrder,
			lastDirectory: currentPath,
			panes: { sidebar: sidebarWidth, details: detailsWidth },
			collapsedSections,
			showDetails,
		};
	}

	// The sidebar is a convenience, so a lookup that fails leaves its group empty
	// rather than blocking the listing behind an error.
	async function loadSidebar() {
		places = await orEmpty(Places());
		devices = await orEmpty(Devices());
		await refreshPins();
		await refreshRecents();
		await refreshTagNames();
		await refreshSavedSearches();
	}

	async function refreshSavedSearches() {
		savedSearches = await orEmpty(SavedSearches());
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

	// A lookup that fails, or answers with nothing, leaves its group as an empty
	// list rather than something that cannot be walked.
	async function orEmpty<Item>(request: Promise<Item[]>): Promise<Item[]> {
		try {
			const loaded = await request;
			if (!loaded) {
				return [];
			}
			return loaded;
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

	// The duplicates of a folder open in a tab of their own, so the folder they
	// were found in is still there to go back to.
	function openDuplicatesPane() {
		if (!currentPath) {
			return;
		}
		closeSearch();
		layout = openTab(layout, { kind: "duplicates", path: currentPath }, nextId);
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

	function goDownloads() {
		const place = places.find((candidate) => candidate.kind === "downloads");
		if (place) {
			openPlace(place.path);
			return;
		}
		openPlace(`${homePath}/Downloads`);
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

	async function openTerminalHere() {
		actionError = "";
		try {
			await OpenTerminal(terminalDirectory());
		} catch (error) {
			actionError = describeError(error);
		}
	}

	// One selected folder is the terminal's directory; anything else means the
	// folder the pane is showing.
	function terminalDirectory(): string {
		if (selectedEntries.length === 1 && selectedEntries[0].isDir) {
			return selectedEntries[0].path;
		}
		return currentPath;
	}

	async function runSearch(query: string, scope: string, mode: "names" | "contents") {
		const sequence = ++latestSearchSequence;
		isSearching = true;
		const outcome = await requestSearch(query, scope, mode);
		if (sequence !== latestSearchSequence) {
			return;
		}
		applySearchOutcome(outcome);
		isSearching = false;
	}

	async function requestSearch(query: string, scope: string, mode: "names" | "contents"): Promise<SearchOutcome> {
		try {
			const result = await searchBy(query, scope, mode);
			return { hits: result.hits, truncated: result.truncated, errorMessage: "" };
		} catch (error) {
			return { hits: [], truncated: false, errorMessage: describeError(error) };
		}
	}

	function searchBy(query: string, scope: string, mode: "names" | "contents"): Promise<proto.SearchResult> {
		if (mode === "contents") {
			return SearchContents(scope, query);
		}
		return Search(query, scope);
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
	async function openSearch(scope: string, mode: "names" | "contents" = "names") {
		searchScope = scope;
		searchMode = mode;
		searchQuery = "";
		isSearchOpen = true;
		await tick();
		searchField?.focus();
	}

	function searchThisFolder() {
		openSearch(currentPath);
	}

	// A saved search opens as a folder of whatever it matches now, which is the
	// whole reason for keeping the query rather than its results.
	function openSavedSearch(search: SavedSearch) {
		closeSearch();
		showLocation({ kind: "search", name: search.name });
	}

	function searchNameOf(location: TabLocation): string {
		if (location.kind !== "search") {
			return "";
		}
		return location.name;
	}

	// A saved search is a name query the index answers; a content search has no such form.
	function startSavingSearch() {
		if (searchQuery.trim() === "" || searchMode === "contents") {
			return;
		}
		searchDialog = {
			title: "Save this search",
			initialName: suggestedName(searchQuery, searchScope),
			description: searchDescription({ name: "", query: searchQuery, scope: searchScope }),
			renaming: null,
		};
	}

	function startRenamingSearch(search: SavedSearch) {
		searchDialog = {
			title: "Rename this search",
			initialName: search.name,
			description: searchDescription(search),
			renaming: search,
		};
	}

	async function commitSearchName(name: string) {
		const dialog = searchDialog;
		searchDialog = null;
		if (!dialog) {
			return;
		}
		await applySearchName(dialog, name);
		await refreshSavedSearches();
	}

	async function applySearchName(dialog: SearchDialogState, name: string) {
		if (dialog.renaming) {
			await renameSavedSearch(dialog.renaming, name);
			return;
		}
		await SaveSearch(name, searchQuery, searchScope);
	}

	// A pane showing the renamed search is looking for it by name, so it is moved
	// along with it rather than left reporting that it no longer exists.
	async function renameSavedSearch(search: SavedSearch, name: string) {
		await RenameSearch(search.name, name);
		if (openSearchName !== search.name) {
			return;
		}
		showLocation({ kind: "search", name });
	}

	async function forgetSavedSearch(name: string) {
		await ForgetSearch(name);
		await refreshSavedSearches();
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

	function pathsHeldByCopy(): string[] {
		if (isClipboardCut) {
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
			paletteOpen: paletteContext !== null,
			selectionCount: selectedEntries.length,
			clipboardCount: clipboardPaths.length,
			entryCount: status.entryCount,
			canUndo: operations.undoLabel !== "",
			canRedo: operations.redoLabel !== "",
			selectionIsArchive: isArchiveSelected(),
			canGoBack: canGoBack(tab.history),
			canGoForward: canGoForward(tab.history),
			canGoUp: status.parentPath !== "",
			viewMode,
			folderPinned: isFolderPinned,
			tabCount: group.tabs.length,
			paneCount: layout.groups.length,
			canSplit: layout.groups.length < maximumGroups,
			directoryOpen: currentPath !== "",
			hoveredIsLink: Boolean(status.hovered?.isSymlink),
		};
	}

	// An operation the engine refused outright has no progress card to report it,
	// so the strip carries it alongside the failures of the chrome's own actions.
	function reportedError(): string {
		if (actionError) {
			return actionError;
		}
		return operations.errorMessage;
	}

	function dismissReportedError() {
		actionError = "";
		operations.errorMessage = "";
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

	// Yazi always shows the preview; the other views show it when asked to.
	function isPreviewShown(paneGroupId: string): boolean {
		if (paneGroupId !== layout.activeGroupId) {
			return false;
		}
		return showDetails || viewMode === "columns";
	}

	// In Yazi's layout the preview is the 3 of 1:4:3; elsewhere it keeps the width it was dragged to.
	function previewShare(): number {
		if (viewMode === "columns") {
			return 3;
		}
		return 0;
	}

	// The bar names the hovered entry itself, so it only adds a summary for a selection of several.
	function selectionSummary(): string {
		if (selectedEntries.length < 2) {
			return "";
		}
		return [strip.primary, strip.secondary].filter(Boolean).join(" · ");
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

	function openSelectionMenu(event: PointerEvent) {
		menu = { x: event.windowX, y: event.windowY, kind: "selection" };
	}

	function openBackgroundMenu(event: PointerEvent) {
		menu = { x: event.windowX, y: event.windowY, kind: "background" };
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

	// The palette runs against the context it opened with, not the one its own
	// input creates by taking focus off the listing.
	function openPalette() {
		paletteContext = { ...commandContext, paletteOpen: true };
	}

	function runPaletteCommand(id: string) {
		const context = paletteContext;
		paletteContext = null;
		if (!context) {
			return;
		}
		registry.run(id, context);
	}

	function paletteShortcutFor(id: string): string {
		if (!paletteContext) {
			return "";
		}
		return shortcutFor(keymap, id, paletteContext);
	}

	function toggleHidden() {
		showHidden = !showHidden;
	}

	function setColumns(ids: ColumnId[]) {
		visibleColumnIds = ids;
	}

	// The backend sorts, so a new order means re-reading the directories rather
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
			"navigate.enter": () => activePane()?.enterFocusedFolder(),
			"navigate.parent": goUp,
			"navigate.back": goBack,
			"navigate.forward": goForward,
			"navigate.home": goHome,
			"navigate.config": () => openPlace(`${homePath}/.config`),
			"navigate.downloads": goDownloads,
			"navigate.trash": openTrash,
			"navigate.terminal": openTerminalHere,
			"navigate.editPath": () => (isEditingPath = true),

			"list.selectAll": () => activePane()?.selectEverything(),
			"list.invertSelection": () => activePane()?.invertSelection(),
			"list.escape": () => activePane()?.escape(),
			"list.toggleSelection": () => activePane()?.toggleFocusedEntry(),
			"list.visual": () => activePane()?.toggleVisual("select"),
			"list.visualUnset": () => activePane()?.toggleVisual("unset"),
			"list.filter": () => activePane()?.openFilter(),
			"list.find": () => activePane()?.openFind("forward"),
			"list.findBackward": () => activePane()?.openFind("backward"),
			"list.findNext": () => activePane()?.findNext("forward"),
			"list.findPrevious": () => activePane()?.findNext("backward"),
			"zoxide.jump": () => (isJumpOpen = true),
			"palette.open": openPalette,

			"tab.new": openNewTab,
			"tab.close": closeActiveTab,
			"tab.next": () => (layout = activateNeighbouringTab(layout, 1)),
			"tab.previous": () => (layout = activateNeighbouringTab(layout, -1)),
			"tab.swapNext": () => (layout = swapActiveTab(layout, 1)),
			"tab.swapPrevious": () => (layout = swapActiveTab(layout, -1)),
			"pane.split": splitPane,
			"pane.close": closeActivePane,
			"pane.focusOther": focusPaneBeside,

			"view.rows": () => setViewMode("rows"),
			"view.grid": () => setViewMode("grid"),
			"view.columns": () => setViewMode("columns"),
			"view.toggleHidden": toggleHidden,
			"view.toggleDetails": () => (showDetails = !showDetails),
			"view.reverseSort": reverseSort,
			"view.toggleDirectoriesFirst": () => setDirectoriesFirst(!sortOrder.directoriesFirst),

			"file.cut": cutSelection,
			"file.copy": copySelection,
			"file.paste": pasteClipboard,
			"file.pasteOverwrite": pasteOverwriting,
			"file.cancelYank": cancelYank,
			"file.create": () => (isCreateDialogOpen = true),
			"file.bulkCreate": () => (isBulkCreateOpen = true),
			"file.rename": renameSelection,
			"preview.seekDown": () => previewPane?.seek(previewSeekLines),
			"preview.seekUp": () => previewPane?.seek(-previewSeekLines),
			"tab.rename": () => (isRenamingTab = true),
			"help.open": () => (isHelpOpen = true),
			"tasks.show": () => (isTaskManagerOpen = true),
			"file.symlink": () => linkClipboard("absolute"),
			"file.symlinkRelative": () => linkClipboard("relative"),
			"file.hardlink": () => linkClipboard("hard"),
			"shell.run": () => (shellPrompt = "background"),
			"shell.runBlocking": () => (shellPrompt = "terminal"),
			"navigate.followLink": followHoveredLink,
			"search.contents": () => openSearch(currentPath, "contents"),
			"list.spot": () => (isSpotOpen = true),
			"file.trash": trashSelection,
			"file.deletePermanently": () => (isConfirmingDelete = true),
			"file.extract": extractSelection,
			"file.compress": compressSelection,
			"file.newFolder": () => activePane()?.createEntry("folder"),
			"file.newFile": () => activePane()?.createEntry("file"),
			"file.undo": () => operations.undo(),
			"file.redo": () => operations.redo(),

			"tags.edit": startTagging,
			"bookmark.pin": pinCurrentFolder,
			"bookmark.unpin": () => unpinFolder(currentPath),

			"search.duplicates": openDuplicatesPane,
			"search.here": searchThisFolder,
			"search.global": searchEverywhere,
			"search.close": closeSearch,
			"search.save": startSavingSearch,
			"app.quit": () => appWindow.close(),
			...generatedHandlers(),
		};
	}

	function generatedHandlers(): Record<string, () => void> {
		const handlers: Record<string, () => void> = {};
		for (const command of sortCommands) {
			handlers[command.id] = () => sortBy(command);
		}
		for (const command of linemodeCommands) {
			handlers[command.id] = () => setColumns(command.columns);
		}
		for (const command of copyTextCommands) {
			handlers[command.id] = () => copySelectionAsText(command.kind);
		}
		for (const command of tabSelectCommands) {
			handlers[command.id] = () => (layout = activateTabAt(layout, command.index));
		}
		return handlers;
	}

	function sortBy(command: SortCommand) {
		sortOrder = sortOrderBy(sortOrder, command.key, command.descending);
		const linemode = linemodeCommands.find((candidate) => candidate.id === command.linemodeId);
		if (linemode) {
			setColumns(linemode.columns);
		}
	}

	async function copySelectionAsText(kind: CopyTextKind) {
		const text = copyTextFor(selectedPathList(), kind);
		if (text === "") {
			return;
		}
		await clipboard.write(text);
	}

	// One entry is renamed in place; several open a buffer of their names.
	function renameSelection() {
		if (selectedEntries.length === 1) {
			activePane()?.startRename();
			return;
		}
		bulkRenameError = "";
		bulkRenamePaths = selectedPathList();
	}

	function applyBulkRename(buffer: string) {
		const plan = renamePlan(bulkRenamePaths, buffer);
		if (plan.errorMessage) {
			bulkRenameError = plan.errorMessage;
			return;
		}
		bulkRenamePaths = [];
		activePane()?.renameMany(plan.pairs);
	}

	function applyBulkCreate(buffer: string) {
		isBulkCreateOpen = false;
		activePane()?.createMany(entriesToCreate(buffer));
	}

	async function linkClipboard(kind: "absolute" | "relative" | "hard") {
		actionError = "";
		try {
			await CreateLinks(clipboardPaths, currentPath, kind);
		} catch (error) {
			actionError = describeError(error);
		}
		reloadPanes();
	}

	// The selection is the command's $1, $2 …, as Yazi passes it.
	async function runShellCommand(command: string) {
		const block = shellPrompt === "terminal";
		shellPrompt = "";
		if (command.trim() === "") {
			return;
		}
		actionError = "";
		try {
			await RunShell(command, currentPath, selectedPathList(), block);
		} catch (error) {
			actionError = describeError(error);
		}
	}

	// The link's target is revealed in the folder holding it, the way a search hit is.
	async function followHoveredLink() {
		const hovered = status.hovered;
		if (!hovered || !hovered.isSymlink) {
			return;
		}
		const destination = linkDestination(hovered.path, hovered.linkTarget);
		closeSearch();
		navigateTo(parentDirectoryOf(destination));
		await tick();
		activePane()?.revealPath(destination);
	}

	function renameActiveTab(title: string) {
		isRenamingTab = false;
		layout = withActiveTabTitle(layout, title);
	}

	function shellPlaceholder(): string {
		if (shellPrompt === "terminal") {
			return "Shell command, run in a terminal";
		}
		return "Shell command";
	}

	function pendingContinuations(): HelpRow[] {
		if (pendingChords.length === 0) {
			return [];
		}
		const applies = (binding: CompiledBinding) => evaluateWhen(binding.when, commandContext);
		return continuationRows(keymap, pendingChords, applies, helpTitleOf);
	}

	function runHelpCommand(command: string) {
		isHelpOpen = false;
		registry.run(command, commandContext);
	}

	function helpTitleOf(command: string): string {
		const registered = registry.byId(command);
		if (!registered) {
			return command;
		}
		return registered.title;
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

	function stepMovement(step: number): Movement {
		if (step < 0) {
			return "up";
		}
		return "down";
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

	function createNamed(kind: "file" | "folder", name: string) {
		isCreateDialogOpen = false;
		activePane()?.createNamed(kind, name);
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

	function pasteClipboard() {
		return paste(false);
	}

	function pasteOverwriting() {
		return paste(true);
	}

	async function paste(replacesExisting: boolean) {
		const sources = await pastableSources();
		if (sources.length === 0) {
			return;
		}
		if (replacesExisting) {
			await operations.startReplacing(pasteKind(), sources, currentPath);
		} else {
			await operations.start(pasteKind(), sources, currentPath);
		}
		clearClipboardAfterMove();
	}

	function cancelYank() {
		clipboardPaths = [];
		isClipboardCut = false;
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

	// Packing lands beside what was packed, which is the folder the pane is on.
	function compressSelection() {
		operations.start("compress", selectedPathList(), currentPath);
	}

	function extractSelection() {
		if (!isArchiveSelected()) {
			return;
		}
		extractArchive(selectedEntries[0].path);
	}

	function isArchiveSelected(): boolean {
		if (selectedEntries.length !== 1) {
			return false;
		}
		return isArchiveName(selectedEntries[0].name);
	}

	// An archive unpacks into a folder of its own beside it, so an archive of
	// loose files does not scatter them over the folder it was in.
	async function extractArchive(archivePath: string) {
		actionError = "";
		try {
			const destination = await ExtractionFolder(archivePath);
			operations.start("extract", [archivePath], destination);
		} catch (error) {
			actionError = describeError(error);
		}
	}

	// Every drop lands here, whichever pane or sidebar item took it, so what a
	// drop is allowed to do is decided in one place.
	function dropOntoFolder(sources: string[], destination: string, operation: DropOperation) {
		if (!isAcceptableDrop(sources, destination, operation)) {
			return;
		}
		operations.start(operation, sources, destination);
		settlePanesAfterDrop();
	}

	function canDragInto(folder: string, sources: string[]): boolean {
		return acceptsInternalDrop(sources, folder, heldModifiers);
	}

	function dropIntoTrash(sources: string[]) {
		operations.start("trash", sources, "");
		settlePanesAfterDrop();
	}

	// The pane a drag left reports its own end, but the row it started on may have
	// been unmounted by then; settling from here does not depend on that event.
	function settlePanesAfterDrop() {
		layout.groups.forEach((paneGroup) => paneFor(paneGroup.id)?.endDrag());
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
		return tagNameOf(target) === "G-BUTTON";
	}

	function isTypingTarget(target: EventTarget | null): boolean {
		return tagNameOf(target) === "G-INPUT";
	}

	// The element a key arrived at is a host element, named by the primitive it draws.
	function tagNameOf(target: EventTarget | null): string {
		if (!target || !("tagName" in target)) {
			return "";
		}
		return String(target.tagName);
	}

	/** Surfaces that take the keyboard for themselves while they are open. */
	function ownsTheKeyboard(): boolean {
		if (Boolean(menu) || isConfirmingDelete || isTagDialogOpen || isCreateDialogOpen) {
			return true;
		}
		if (isBulkCreateOpen || bulkRenamePaths.length > 0 || isRenamingTab || isHelpOpen) {
			return true;
		}
		if (isTaskManagerOpen || isSpotOpen || shellPrompt !== "") {
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
		if (searchMode === "contents") {
			return "Search contents of this folder";
		}
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

	function openTagName(): string {
		if (tab.location.kind !== "tag") {
			return "";
		}
		return tab.location.name;
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
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#snippet sidebar()}
	{#if isSidebarOpen}
		<PlacesSidebar
			{places}
			{devices}
			{pinned}
			{recent}
			{tagNames}
			{savedSearches}
			{openSearchName}
			{currentPath}
			isTrashOpen={tab.location.kind === "trash"}
			openTag={openTagName()}
			{collapsedSections}
			width={sidebarWidth}
			onToggleSection={toggleSection}
			onNavigate={openPlace}
			onOpenRecent={openRecent}
			onOpenTrash={openTrash}
			onOpenTag={openTagPane}
			onRemoveBookmark={unpinFolder}
			onForgetTag={forgetTag}
			onOpenSearch={openSavedSearch}
			onRenameSearch={startRenamingSearch}
			onForgetSearch={forgetSavedSearch}
			onDropOnFolder={dropOntoFolder}
			onDropInTrash={dropIntoTrash}
		/>
		<PaneResizer
			side="left"
			width={sidebarWidth}
			label="Resize the places sidebar"
			onWidth={(width) => (sidebarWidth = width)}
		/>
	{/if}
{/snippet}

{#snippet locationBar()}
	<View
		style={{
			direction: "row",
			height: headerHeight,
			shrink: 0,
			align: "center",
			gap: space[2],
			paddingX: space[2],
			borderBottomWidth: 1,
			borderColor: colors.line,
		}}
	>
		<Button size="sm" variant="ghost" icon={caretLeft} disabled={!canGoBack(tab.history)} onpress={goBack} />
		<Button size="sm" variant="ghost" icon={caretRight} disabled={!canGoForward(tab.history)} onpress={goForward} />
		{#if isSearchOpen}
			<SearchField
				bind:this={searchField}
				bind:value={searchQuery}
				placeholder={searchPlaceholder()}
				scope={scopeLabel()}
				hits={searchHits}
				truncated={searchTruncated}
				errorMessage={searchErrorMessage}
				indexingLabel={indexingNotice()}
				{isSearching}
				onOpenHit={openHit}
				onSave={startSavingSearch}
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
			{showDetails}
			onShowDetailsChange={(shown) => (showDetails = shown)}
		/>
	</View>
{/snippet}

{#snippet paneGroups()}
	<View style={{ direction: "row", flex: 1, minHeight: 0 }}>
		{#each layout.groups as paneGroup, index (paneGroup.id)}
			{#if index > 0}
				<View style={{ width: 1, shrink: 0, background: colors.line }} />
			{/if}
			<View style={{ flex: 1, minWidth: 0 }}>
				<FilePane
					bind:this={panes[paneGroup.id]}
					location={activeTab(paneGroup).location}
					{viewMode}
					{visibleColumnIds}
					{sortOrder}
					{showHidden}
					{cutPaths}
					{copiedPaths}
					{savedSearches}
					isActive={paneGroup.id === layout.activeGroupId}
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
					onChanged={() => operations.refreshHistoryLabels()}
					onDropPaths={dropOntoFolder}
					onExtractArchive={extractArchive}
				>
					{#snippet header()}
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
					{/snippet}
					{#snippet aside()}
						{#if isPreviewShown(paneGroup.id)}
							{#if viewMode === "columns"}
								<View style={{ width: 1, shrink: 0, background: colors.line }} />
							{:else}
								<PaneResizer
									side="right"
									width={detailsWidth}
									label="Resize the details pane"
									onWidth={(width) => (detailsWidth = width)}
								/>
							{/if}
							<PreviewPane
								bind:this={previewPane}
								entries={selectedEntries}
								width={detailsWidth}
								share={previewShare()}
								{sortOrder}
								{showHidden}
								onOpenFolder={navigateTo}
								canDropInto={canDragInto}
								onDropPaths={(folder, sources) =>
									dropOntoFolder(sources, folder, dropOperation("internal", heldModifiers))}
							/>
						{/if}
					{/snippet}
				</FilePane>
			</View>
		{/each}
	</View>
{/snippet}

<!-- Everything sits flush against the window's edges, divided by hairlines. -->
<View style={root}>
	<View style={{ direction: "row", flex: 1, minHeight: 0 }}>
		{@render sidebar()}
		<View style={{ flex: 1, minWidth: 0, minHeight: 0 }}>
			{@render locationBar()}
			{@render paneGroups()}
		</View>
	</View>

	<StatusBar {status} summary={selectionSummary()} errorMessage={reportedError()} onDismissError={dismissReportedError} />

	<!-- Later siblings draw on top, so the progress cards come before the dialogs. -->
	<OperationProgress
		operations={operations.operations}
		onPause={(id) => operations.pause(id)}
		onResume={(id) => operations.resume(id)}
		onCancel={(id) => operations.cancel(id)}
		onDismiss={(id) => operations.dismiss(id)}
	/>

	{#if paletteContext}
		<CommandPalette
			commands={registry.all()}
			context={paletteContext}
			shortcutOf={paletteShortcutFor}
			onRun={runPaletteCommand}
			onClose={() => (paletteContext = null)}
		/>
	{/if}

	{#if isJumpOpen}
		<JumpField {currentPath} onJump={jumpToFolder} onClose={() => (isJumpOpen = false)} />
	{/if}

	{#if menu}
		<ContextMenu x={menu.x} y={menu.y} groups={menuGroupsFor(menu.kind)} onSelect={runCommand} onClose={closeMenu} />
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

	{#if isCreateDialogOpen}
		<CreateEntryDialog onCreate={createNamed} onClose={() => (isCreateDialogOpen = false)} />
	{/if}

	{#if isBulkCreateOpen}
		<LinesDialog
			title="Create several"
			hint="One name per line; end a name with / to make a folder. Ctrl+Enter creates."
			submitLabel="Create"
			onSubmit={applyBulkCreate}
			onClose={() => (isBulkCreateOpen = false)}
		/>
	{/if}

	{#if bulkRenamePaths.length > 0}
		<LinesDialog
			title="Rename {bulkRenamePaths.length} entries"
			hint="Edit the names in place, one per line. Ctrl+Enter renames."
			initialText={renameBuffer(bulkRenamePaths)}
			errorMessage={bulkRenameError}
			submitLabel="Rename"
			onSubmit={applyBulkRename}
			onClose={() => (bulkRenamePaths = [])}
		/>
	{/if}

	{#if isRenamingTab}
		<TextPrompt
			icon={textT}
			placeholder="Tab name"
			hint="Leave empty to name the tab after its folder again."
			initialValue={tab.title}
			onSubmit={renameActiveTab}
			onClose={() => (isRenamingTab = false)}
		/>
	{/if}

	{#if shellPrompt !== ""}
		<TextPrompt
			icon={terminalWindow}
			placeholder={shellPlaceholder()}
			hint="Runs in this folder; the selected entries are $1, $2 … and $@."
			onSubmit={runShellCommand}
			onClose={() => (shellPrompt = "")}
		/>
	{/if}

	{#if pendingChords.length > 0 && whichKeyRows.length > 0}
		<WhichKey prefix={sequenceLabel(pendingChords)} rows={whichKeyRows} />
	{/if}

	{#if isTaskManagerOpen}
		<TaskManager
			operations={operations.operations}
			onCancel={(id) => operations.cancel(id)}
			onClose={() => (isTaskManagerOpen = false)}
		/>
	{/if}

	{#if isSpotOpen && status.hovered}
		<SpotDialog
			entry={status.hovered}
			onSwipe={(step) => moveSelection(stepMovement(step), false)}
			onClose={() => (isSpotOpen = false)}
		/>
	{/if}

	{#if isHelpOpen}
		<HelpPanel rows={helpRows(keymap, helpTitleOf)} onRun={runHelpCommand} onClose={() => (isHelpOpen = false)} />
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

	{#if searchDialog}
		<SaveSearchDialog
			title={searchDialog.title}
			initialName={searchDialog.initialName}
			description={searchDialog.description}
			saved={savedSearches}
			onSave={commitSearchName}
			onClose={() => (searchDialog = null)}
		/>
	{/if}

	{#if openWithPath}
		<OpenWithDialog path={openWithPath} onOpen={openWithApplication} onCancel={() => (openWithPath = "")} />
	{/if}

</View>
