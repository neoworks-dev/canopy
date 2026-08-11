<script lang="ts">
	import { onMount, untrack } from "svelte";
	import { Button, Card, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import ArrowLeftIcon from "phosphor-svelte/lib/ArrowLeftIcon";
	import ArrowRightIcon from "phosphor-svelte/lib/ArrowRightIcon";
	import ArrowUpIcon from "phosphor-svelte/lib/ArrowUpIcon";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import EyeIcon from "phosphor-svelte/lib/EyeIcon";
	import EyeSlashIcon from "phosphor-svelte/lib/EyeSlashIcon";
	import ListIcon from "phosphor-svelte/lib/ListIcon";
	import SquaresFourIcon from "phosphor-svelte/lib/SquaresFourIcon";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import {
		CopyPathsToClipboard,
		CreateFile,
		CreateFolder,
		ListDirectory,
		LoadSettings,
		OpenPath,
		OpenWith,
		Places,
		ReadClipboardPaths,
		RenameEntry,
		SaveSettings,
		SetDefaultApplication,
		StopWatching,
		SuggestName,
		WatchDirectory,
	} from "../wailsjs/go/main/FileService";
	import { IndexStatus, Search } from "../wailsjs/go/main/IndexService";
	import { EventsOn } from "../wailsjs/runtime/runtime";
	import { settings as storedSettings } from "../wailsjs/go/models";
	import type { filesystem, proto } from "../wailsjs/go/models";
	import ColumnMenu from "./lib/ColumnMenu.svelte";
	import ConfirmDialog from "./lib/ConfirmDialog.svelte";
	import ConflictDialog from "./lib/ConflictDialog.svelte";
	import ContextMenu from "./lib/ContextMenu.svelte";
	import EntryGrid from "./lib/EntryGrid.svelte";
	import EntryRows from "./lib/EntryRows.svelte";
	import FindField from "./lib/FindField.svelte";
	import OpenWithDialog from "./lib/OpenWithDialog.svelte";
	import OperationProgress from "./lib/OperationProgress.svelte";
	import PathBar from "./lib/PathBar.svelte";
	import PlacesSidebar from "./lib/PlacesSidebar.svelte";
	import PreviewPane from "./lib/PreviewPane.svelte";
	import SearchField from "./lib/SearchField.svelte";
	import SearchResults from "./lib/SearchResults.svelte";
	import TrashPane from "./lib/TrashPane.svelte";
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
	import { columnsFor, defaultColumnIds, type ColumnId } from "./lib/columns";
	import { entryElementId, indexOfOption } from "./lib/entryList";
	import { describeError } from "./lib/errors";
	import { parentDirectoryOf } from "./lib/format";
	import {
		canGoBack,
		canGoForward,
		currentLocation,
		emptyHistory,
		historyAfterBack,
		historyAfterForward,
		historyAfterVisit,
	} from "./lib/history";
	import {
		exceedsThreshold,
		indexesWithin,
		rectangleBetween,
		type Point,
		type Rectangle,
	} from "./lib/marquee";
	import { targetIndexFor, type Movement } from "./lib/movement";
	import {
		defaultViewSettings,
		viewSettingsFrom,
		type ViewMode,
	} from "./lib/settings";
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
		emptySelection,
		pruneSelection,
		selectAll,
		selectionAfterClick,
		selectionAfterJump,
		selectionAfterRelisting,
		selectionFromIndexes,
		type Selection,
	} from "./lib/selection";
	import {
		listWindow,
		pageSize,
		sameMetrics,
		scrollTopRevealing,
		unmeasuredMetrics,
		type ListMetrics,
		type Viewport,
	} from "./lib/virtualList";
	import { directoryChangedEvent } from "./lib/watch";

	// Long enough that typing a word does not fire a query per keystroke, short
	// enough that results feel immediate.
	const searchDebounceMilliseconds = 150;
	const statusPollMilliseconds = 3000;

	type SearchOutcome = {
		hits: proto.Hit[];
		truncated: boolean;
		errorMessage: string;
	};

	type MenuState = {
		x: number;
		y: number;
		groups: MenuItem[][];
	};

	let listing = $state<filesystem.Listing | null>(null);
	let places = $state<filesystem.Place[]>([]);
	let errorMessage = $state("");
	// Failures of one action, reported above the listing instead of replacing it.
	let actionError = $state("");
	let isLoading = $state(false);
	let showHidden = $state(false);
	let viewMode = $state<ViewMode>("rows");
	let visibleColumnIds = $state<ColumnId[]>(defaultColumnIds);
	let sortOrder = $state<SortOrder>(defaultSortOrder);
	// Nothing is written back until the stored settings have been applied, or the
	// first save would overwrite them with defaults.
	let areSettingsLoaded = $state(false);
	let selection = $state<Selection>(emptySelection);
	let history = $state(emptyHistory);
	let isEditingPath = $state(false);
	let isTrashOpen = $state(false);
	let listMetrics = $state<ListMetrics>(unmeasuredMetrics);
	let viewport = $state<Viewport>({ scrollTop: 0, height: 0 });
	let listContainer = $state<HTMLDivElement>();
	let searchInput = $state<HTMLInputElement>();

	let clipboardPaths = $state<string[]>([]);
	let isClipboardCut = $state(false);

	let menu = $state<MenuState | null>(null);
	let renamingPath = $state("");
	let isConfirmingDelete = $state(false);
	let openWithPath = $state("");
	let isFindOpen = $state(false);
	// What the selection was before the find prompt started moving it, so
	// cancelling puts it back.
	let findOrigin: Selection = emptySelection;
	// Chords typed so far in a sequence such as "g g", waiting for the rest.
	let pendingChords = $state<string[]>([]);
	let userKeybindings = $state<Keybinding[]>([]);

	let band = $state<Rectangle | null>(null);
	let bandOrigin: Point = { x: 0, y: 0 };
	let bandKeep: string[] = [];

	let searchQuery = $state("");
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

	const operations = new OperationQueue(() => reloadListing());
	const registry = new CommandRegistry();
	registry.registerAll(builtinCommandList());

	const currentPath = $derived(pathOf(listing));
	const canGoUp = $derived(Boolean(listing?.parentPath));
	const visibleEntries = $derived.by(collectVisibleEntries);
	const visiblePaths = $derived(visibleEntries.map((entry) => entry.path));
	const selectedPaths = $derived(new Set(selection.paths));
	const selectedEntries = $derived(visibleEntries.filter((entry) => selectedPaths.has(entry.path)));
	const activeColumns = $derived(columnsFor(visibleColumnIds));
	const isSearchActive = $derived(searchQuery.trim().length > 0);
	const isIndexing = $derived(indexStatus?.state === "scanning");
	const commandContext = $derived(currentCommandContext());
	// The user's bindings come last, so any of them overrides a built-in.
	const keymap = $derived(compileKeymap([...defaultKeymap, ...userKeybindings]));
	const conflict = $derived(operations.conflict);
	// Only a cut marks entries as pending; a copy leaves them as they are.
	const cutPaths = $derived(new Set(pathsHeldByCut()));
	// The trash and search panes replace the listing, so nothing is watched then.
	const watchedPath = $derived(directoryOnScreen());
	// Only the entries around the viewport are mounted, so a directory of a
	// hundred thousand builds a screenful of DOM rather than all of it.
	const mountedWindow = $derived(listWindow(listMetrics, visibleEntries.length, viewport));
	const mountedEntries = $derived(
		visibleEntries.slice(mountedWindow.firstIndex, mountedWindow.endIndex),
	);

	onMount(() => {
		start();
		loadPlaces();
		const stopOperations = operations.listen();
		const stopWatching = EventsOn(directoryChangedEvent, applyExternalChange);
		return () => {
			stopOperations();
			stopWatching();
		};
	});

	$effect(() => {
		followDirectory(watchedPath);
	});

	$effect(() => {
		const next = currentViewSettings();
		if (!areSettingsLoaded) {
			return;
		}
		SaveSettings(next);
	});

	$effect(() => {
		const query = searchQuery.trim();
		const scope = searchScope;
		if (!query) {
			clearSearch();
			return;
		}
		selection = emptySelection;
		const timer = setTimeout(() => runSearch(query, scope), searchDebounceMilliseconds);
		return () => clearTimeout(timer);
	});

	// Hiding dotfiles must not leave invisible entries selected.
	$effect(() => {
		selection = pruneSelection(selection, visiblePaths);
	});

	// Keyboard movement is useless if the entry it lands on is off screen. Only
	// the focus is a reason to scroll: reacting to the viewport as well would drag
	// the focused entry back every time the user scrolled away from it.
	$effect(() => {
		const index = selection.focusIndex;
		untrack(() => revealEntry(index));
	});

	// The listing is the default keyboard target, so arrows work without a click
	// first — but only while nothing else has claimed focus.
	$effect(() => {
		claimInitialFocus(listContainer);
	});

	// The window depends on how much of the listing fits, which changes with the
	// window and with the panes beside it.
	$effect(() => {
		if (!listContainer) {
			return;
		}
		const container = listContainer;
		const observer = new ResizeObserver(() => measureViewport(container));
		observer.observe(container);
		measureViewport(container);
		return () => observer.disconnect();
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

	// The stored settings decide how the first listing is shown, so they are
	// applied before it is read.
	async function start() {
		const restored = await restoreSettings();
		await navigateTo(restored.lastDirectory);
		fallBackHome();
	}

	// A stored directory that has since been deleted must not leave the window
	// sitting on an error card at launch.
	function fallBackHome() {
		if (!errorMessage) {
			return;
		}
		navigateTo("");
	}

	async function restoreSettings() {
		const stored = await loadStoredSettings();
		viewMode = stored.viewMode;
		visibleColumnIds = stored.columns;
		showHidden = stored.showHidden;
		sortOrder = stored.sort;
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
		});
	}

	async function navigateTo(path: string) {
		isTrashOpen = false;
		isEditingPath = false;
		isLoading = true;
		errorMessage = "";
		selection = emptySelection;
		try {
			listing = await ListDirectory(path, sortOrder);
			history = historyAfterVisit(history, listing.path);
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
	}

	/** Re-reads the current directory in place, keeping history and selection. */
	async function reloadListing() {
		if (!currentPath) {
			return;
		}
		const previousPaths = visiblePaths;
		try {
			listing = await ListDirectory(currentPath, sortOrder);
		} catch (error) {
			errorMessage = describeError(error);
			return;
		}
		selection = selectionAfterRelisting(selection, previousPaths, visiblePaths);
	}

	function directoryOnScreen(): string {
		if (isTrashOpen || isSearchActive) {
			return "";
		}
		return currentPath;
	}

	async function followDirectory(path: string) {
		if (!path) {
			await StopWatching();
			return;
		}
		try {
			await WatchDirectory(path);
		} catch {
			// A directory that cannot be watched still lists, it just stays a snapshot.
		}
	}

	// A change reported for a directory the user has already left is stale.
	function applyExternalChange(path: string) {
		if (path !== currentPath) {
			return;
		}
		reloadListing();
	}

	// The sidebar is a convenience, so a failed lookup hides it rather than
	// blocking the listing behind an error.
	async function loadPlaces() {
		try {
			places = await Places();
		} catch {
			places = [];
		}
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

	function clearSearch() {
		latestSearchSequence++;
		searchScope = "";
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

	function openHit(hit: proto.Hit) {
		searchQuery = "";
		if (hit.isDir) {
			navigateTo(hit.path);
			return;
		}
		navigateTo(parentDirectoryOf(hit.path));
		launch(hit.path);
	}

	function collectVisibleEntries(): filesystem.Entry[] {
		if (!listing) {
			return [];
		}
		if (showHidden) {
			return listing.entries;
		}
		return listing.entries.filter((entry) => !entry.name.startsWith("."));
	}

	function selectEntry(index: number, event: MouseEvent) {
		selection = selectionAfterClick(selection, visiblePaths, index, {
			toggle: event.ctrlKey || event.metaKey,
			range: event.shiftKey,
		});
	}

	function openEntry(entry: filesystem.Entry) {
		if (entry.isDir) {
			navigateTo(entry.path);
			return;
		}
		launch(entry.path);
	}

	async function launch(path: string) {
		try {
			await OpenPath(path);
		} catch (error) {
			actionError = describeError(error);
		}
	}

	// Computed rather than scrolled into view, because the entry the keyboard just
	// moved to may not be mounted yet.
	function revealEntry(index: number) {
		if (!listContainer) {
			return;
		}
		const target = scrollTopRevealing(listMetrics, index, viewport);
		if (target === viewport.scrollTop) {
			return;
		}
		listContainer.scrollTo({ top: target, behavior: "smooth" });
	}

	function applyMetrics(reported: ListMetrics) {
		if (sameMetrics(listMetrics, reported)) {
			return;
		}
		listMetrics = reported;
	}

	function trackScroll(event: Event & { currentTarget: HTMLDivElement }) {
		viewport = { scrollTop: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight };
	}

	function measureViewport(container: HTMLDivElement) {
		viewport = { scrollTop: container.scrollTop, height: container.clientHeight };
	}

	function claimInitialFocus(container: HTMLDivElement | undefined) {
		if (!container || document.activeElement !== document.body) {
			return;
		}
		container.focus();
	}

	function focusSearch() {
		searchInput?.focus();
		searchInput?.select();
	}

	// The scope is set before the field takes focus, so the first keystroke
	// already queries the right thing.
	function searchThisFolder() {
		searchScope = currentPath;
		focusSearch();
	}

	function searchEverywhere() {
		searchScope = "";
		focusSearch();
	}

	function activeDescendant(): string | undefined {
		if (selection.focusIndex < 0) {
			return undefined;
		}
		return entryElementId(selection.focusIndex);
	}

	function pathsHeldByCut(): string[] {
		if (!isClipboardCut) {
			return [];
		}
		return clipboardPaths;
	}

	function currentCommandContext(): WhenContext {
		return {
			listActive: isListActive(),
			searchOpen: isSearchActive,
			trashOpen: isTrashOpen,
			findOpen: isFindOpen,
			selectionCount: selectedEntries.length,
			clipboardCount: clipboardPaths.length,
			entryCount: visibleEntries.length,
			canUndo: operations.undoLabel !== "",
			canGoBack: canGoBack(history),
			canGoForward: canGoForward(history),
			canGoUp,
			viewMode,
		};
	}

	/** The listing is the pane on screen, rather than trash, search or an error. */
	function isListActive(): boolean {
		return !isTrashOpen && !isSearchActive && errorMessage === "" && !isLoading;
	}

	function selectionMenuGroups(): MenuItem[][] {
		return menuGroups(selectionMenuIds);
	}

	function backgroundMenuGroups(): MenuItem[][] {
		return menuGroups(backgroundMenuIds);
	}

	function menuGroups(layout: string[][]): MenuItem[][] {
		return layout.map((group) => group.flatMap(menuItemFor));
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

	function goUp() {
		if (!listing?.parentPath) {
			return;
		}
		navigateTo(listing.parentPath);
	}

	function goHome() {
		searchQuery = "";
		navigateTo("");
	}

	function goBack() {
		const moved = historyAfterBack(history);
		if (moved === history) {
			return;
		}
		history = moved;
		openHistoryLocation();
	}

	function goForward() {
		const moved = historyAfterForward(history);
		if (moved === history) {
			return;
		}
		history = moved;
		openHistoryLocation();
	}

	// Loads what the history now points at without recording another visit.
	async function openHistoryLocation() {
		isTrashOpen = false;
		selection = emptySelection;
		errorMessage = "";
		try {
			listing = await ListDirectory(currentLocation(history), sortOrder);
		} catch (error) {
			errorMessage = describeError(error);
		}
	}

	function openPlace(path: string) {
		searchQuery = "";
		navigateTo(path);
	}

	function openTrash() {
		searchQuery = "";
		isTrashOpen = true;
		selection = emptySelection;
	}

	function toggleHidden() {
		showHidden = !showHidden;
	}

	function setColumns(ids: ColumnId[]) {
		visibleColumnIds = ids;
	}

	// The Go side sorts, so a new order means re-reading the directory rather than
	// reordering what is on screen.
	function sortByColumn(key: SortKey) {
		sortOrder = sortOrderAfterHeaderClick(sortOrder, key);
		reloadListing();
	}

	function setDirectoriesFirst(directoriesFirst: boolean) {
		sortOrder = sortOrderWithDirectoriesFirst(sortOrder, directoriesFirst);
		reloadListing();
	}

	// The two views have different geometry, so the old measurements are dropped
	// and the incoming view reports its own.
	function showRows() {
		viewMode = "rows";
		listMetrics = unmeasuredMetrics;
	}

	function showGrid() {
		viewMode = "grid";
		listMetrics = unmeasuredMetrics;
	}

	function openContextMenu(index: number, event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();
		selectForMenu(index);
		menu = { x: event.clientX, y: event.clientY, groups: selectionMenuGroups() };
	}

	// Right-clicking outside the selection moves it, the way every file manager
	// behaves; right-clicking inside keeps the multi-selection intact.
	function selectForMenu(index: number) {
		if (selectedPaths.has(visiblePaths[index])) {
			return;
		}
		selection = selectionAfterJump(selection, visiblePaths, index, false);
	}

	function openBackgroundMenu(event: MouseEvent) {
		event.preventDefault();
		menu = { x: event.clientX, y: event.clientY, groups: backgroundMenuGroups() };
	}

	function closeMenu() {
		menu = null;
	}

	function runCommand(id: string) {
		closeMenu();
		registry.run(id, commandContext);
	}

	function commandHandlers(): Record<string, () => void> {
		return {
			"navigate.open": openSelection,
			"navigate.openWith": startOpenWith,
			"navigate.parent": goUp,
			"navigate.back": goBack,
			"navigate.forward": goForward,
			"navigate.home": goHome,
			"navigate.trash": openTrash,
			"navigate.editPath": () => (isEditingPath = true),

			"list.selectAll": () => (selection = selectAll(visiblePaths)),
			"list.clearSelection": () => (selection = emptySelection),
			"list.toggleSelection": toggleFocusedEntry,
			"list.find": openFind,

			"view.rows": showRows,
			"view.grid": showGrid,
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
			"file.rename": startRename,
			"file.trash": trashSelection,
			"file.deletePermanently": () => (isConfirmingDelete = true),
			"file.newFolder": () => createEntry("folder"),
			"file.newFile": () => createEntry("file"),
			"file.undo": () => operations.undo(),

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

	// A built-in without a handler is a wiring mistake, and one that would show up
	// as a shortcut that quietly does nothing.
	function handlerFor(handlers: Record<string, () => void>, id: string): () => void {
		const handler = handlers[id];
		if (!handler) {
			throw new Error(`command ${id} has no handler`);
		}
		return handler;
	}

	function openSelection() {
		selectedEntries.forEach(openEntry);
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
		clipboardPaths = selectedEntries.map((entry) => entry.path);
		isClipboardCut = false;
		publishClipboard();
	}

	function cutSelection() {
		clipboardPaths = selectedEntries.map((entry) => entry.path);
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

	function startRename() {
		if (selectedEntries.length !== 1) {
			return;
		}
		renamingPath = selectedEntries[0].path;
	}

	async function commitRename(name: string) {
		const path = renamingPath;
		renamingPath = "";
		actionError = "";
		try {
			await RenameEntry(path, name);
		} catch (error) {
			actionError = describeError(error);
		}
		await reloadListing();
		await operations.refreshUndoLabel();
	}

	function cancelRename() {
		renamingPath = "";
	}

	// A new entry is created under a free name and then immediately handed to the
	// inline editor, so naming it is one uninterrupted gesture.
	async function createEntry(kind: "folder" | "file") {
		actionError = "";
		try {
			const created = await createAndReveal(kind);
			selection = selectionAfterJump(emptySelection, [created], 0, false);
			renamingPath = created;
		} catch (error) {
			actionError = describeError(error);
		}
		await operations.refreshUndoLabel();
	}

	async function createAndReveal(kind: "folder" | "file"): Promise<string> {
		const name = await SuggestName(currentPath, defaultNameFor(kind));
		const created = await createOfKind(kind, name);
		await reloadListing();
		return created;
	}

	function createOfKind(kind: "folder" | "file", name: string): Promise<string> {
		if (kind === "folder") {
			return CreateFolder(currentPath, name);
		}
		return CreateFile(currentPath, name);
	}

	function defaultNameFor(kind: "folder" | "file"): string {
		if (kind === "folder") {
			return "New folder";
		}
		return "New file";
	}

	// A drag that starts on an entry is a click on that entry, and one that starts
	// on a control belongs to that control; only the space around them opens a
	// rubber band. Capturing the pointer here would otherwise redirect the
	// pointerup, and the control would never see a click at all.
	function startBand(event: PointerEvent) {
		if (event.button !== 0 || startsOnInteractive(event)) {
			return;
		}
		bandOrigin = { x: event.clientX, y: event.clientY };
		bandKeep = keptDuringBand(event);
		selection = bandSelection([]);
		band = rectangleBetween(bandOrigin, bandOrigin);
		listContainer?.setPointerCapture(event.pointerId);
	}

	function startsOnInteractive(event: PointerEvent): boolean {
		if (!(event.target instanceof HTMLElement)) {
			return false;
		}
		return Boolean(event.target.closest('[role="option"], button, input'));
	}

	function keptDuringBand(event: PointerEvent): string[] {
		if (event.ctrlKey || event.metaKey) {
			return [...selection.paths];
		}
		return [];
	}

	function updateBand(event: PointerEvent) {
		if (!band) {
			return;
		}
		const current = rectangleBetween(bandOrigin, { x: event.clientX, y: event.clientY });
		band = current;
		selection = bandSelection(indexesCoveredBy(current));
	}

	// A band can only cover what is on screen, so hit testing runs over the
	// mounted entries and reads back the index each one carries.
	function indexesCoveredBy(current: Rectangle): number[] {
		const options = mountedOptions();
		const covered = indexesWithin(current, options.map((option) => option.getBoundingClientRect()));
		return covered.map((position) => indexOfOption(options[position]));
	}

	function endBand(event: PointerEvent) {
		if (!band) {
			return;
		}
		listContainer?.releasePointerCapture(event.pointerId);
		clearSelectionAfterClick();
		band = null;
	}

	// A pointer that never really moved was a click on the background, which
	// clears the selection rather than selecting nothing in particular.
	function clearSelectionAfterClick() {
		if (band && exceedsThreshold(band)) {
			return;
		}
		selection = emptySelection;
	}

	function bandSelection(indexes: number[]): Selection {
		return selectionFromIndexes(visiblePaths, indexes, bandKeep);
	}

	function mountedOptions(): Element[] {
		if (!listContainer) {
			return [];
		}
		return [...listContainer.querySelectorAll('[role="option"]')];
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
		return Boolean(menu) || isConfirmingDelete || openWithPath !== "" || Boolean(conflict);
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

	function moveSelection(movement: Movement, extend: boolean) {
		const target = targetIndexFor(movement, selection.focusIndex, movementContext());
		if (target < 0) {
			return;
		}
		selection = selectionAfterJump(selection, visiblePaths, target, extend);
	}

	function movementContext() {
		return {
			total: visiblePaths.length,
			columns: listMetrics.columns,
			pageSize: pageSize(listMetrics, viewport),
		};
	}

	function toggleFocusedEntry() {
		if (selection.focusIndex < 0) {
			return;
		}
		selection = selectionAfterClick(selection, visiblePaths, selection.focusIndex, {
			toggle: true,
			range: false,
		});
	}

	function openFind() {
		findOrigin = selection;
		isFindOpen = true;
	}

	function closeFind(accepted: boolean) {
		isFindOpen = false;
		if (!accepted) {
			selection = findOrigin;
		}
		listContainer?.focus();
	}

	function jumpToFound(index: number) {
		selection = selectionAfterJump(selection, visiblePaths, index, false);
	}

	function closeSearch() {
		searchQuery = "";
		listContainer?.focus();
	}

	function searchPlaceholder(): string {
		if (!searchScope) {
			return "Search everywhere";
		}
		return `Search in ${nameOfDirectory(searchScope)}`;
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

	function reverseSort() {
		sortOrder = sortOrderReversed(sortOrder);
		reloadListing();
	}

	function viewVariant(mode: ViewMode): "surface" | "ghost" {
		if (viewMode === mode) {
			return "surface";
		}
		return "ghost";
	}

	function hiddenToggleIcon(): IconComponent {
		if (showHidden) {
			return EyeIcon;
		}
		return EyeSlashIcon;
	}

	function pathOf(current: filesystem.Listing | null): string {
		if (!current) {
			return "";
		}
		return current.path;
	}

	function itemCountLabel(count: number): string {
		if (count === 1) {
			return "1 item";
		}
		return `${count} items`;
	}

	function listingSummary(): string {
		const items = itemCountLabel(visibleEntries.length);
		if (selection.paths.length === 0) {
			return items;
		}
		return `${items} · ${selection.paths.length} selected`;
	}

	function indexingLabel(status: proto.StatusResult): string {
		return `Indexing ${status.filesIndexed.toLocaleString()} entries so far`;
	}

	function deleteMessage(): string {
		return `${itemCountLabel(selectedEntries.length)} will be deleted permanently. This cannot be undone.`;
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
	<Card surface="elevated" padding="none" class="flex shrink-0 items-center gap-2 p-2 shadow-lg">
		<Button
			size="sm"
			variant="ghost"
			icon={ArrowLeftIcon}
			disabled={!canGoBack(history)}
			onclick={goBack}
		/>
		<Button
			size="sm"
			variant="ghost"
			icon={ArrowRightIcon}
			disabled={!canGoForward(history)}
			onclick={goForward}
		/>
		<Button size="sm" variant="ghost" icon={ArrowUpIcon} disabled={!canGoUp} onclick={goUp} />
		<Button size="sm" variant="ghost" icon={HouseIcon} onclick={goHome} />

		<PathBar
			path={currentPath}
			isEditing={isEditingPath}
			onNavigate={navigateTo}
			onEditingChange={(editing) => (isEditingPath = editing)}
		/>

		<SearchField
			bind:value={searchQuery}
			bind:input={searchInput}
			placeholder={searchPlaceholder()}
		/>

		<Button size="sm" variant={viewVariant("rows")} icon={ListIcon} onclick={showRows} />
		<Button size="sm" variant={viewVariant("grid")} icon={SquaresFourIcon} onclick={showGrid} />

		<ColumnMenu
			visibleIds={visibleColumnIds}
			onChange={setColumns}
			directoriesFirst={sortOrder.directoriesFirst}
			onDirectoriesFirstChange={setDirectoriesFirst}
		/>

		<Button size="sm" variant="ghost" icon={hiddenToggleIcon()} onclick={toggleHidden} />
	</Card>

	<div class="flex min-h-0 flex-1 gap-2">
		{#if places.length > 0}
			<PlacesSidebar
				{places}
				{currentPath}
				{isTrashOpen}
				onNavigate={openPlace}
				onOpenTrash={openTrash}
			/>
		{/if}

		<Card
			surface="elevated"
			padding="none"
			class="flex min-w-0 flex-1 flex-col gap-2 overflow-hidden p-2 shadow-lg"
		>
			{#if isTrashOpen}
				<div class="flex min-h-0 flex-col gap-2 overflow-y-auto">
					<TrashPane onRestored={() => reloadListing()} />
				</div>
			{:else if isSearchActive}
				<div class="flex min-h-0 flex-col gap-2 overflow-y-auto">
					{#if searchErrorMessage}
						<Card surface="raised" padding="none" class="flex items-center gap-2 p-2">
							<WarningIcon size={20} class="shrink-0 text-red" />
							<div class="min-w-0">
								<p class="text-sm font-medium text-default">Search is unavailable</p>
								<p class="truncate text-xs text-dim">{searchErrorMessage}</p>
							</div>
						</Card>
					{:else if isSearching && searchHits.length === 0}
						<div class="flex justify-center p-2">
							<LoadingSpinner />
						</div>
					{:else}
						{#if isIndexing && indexStatus}
							<p class="text-xs text-dim">{indexingLabel(indexStatus)}</p>
						{/if}
						{#if searchHits.length === 0}
							<SectionHeader title="No matches" />
						{:else}
							<SearchResults
							hits={searchHits}
							truncated={searchTruncated}
							scope={scopeLabel()}
							onOpen={openHit}
						/>
						{/if}
					{/if}
				</div>
			{:else if errorMessage}
				<Card surface="raised" padding="none" class="flex items-center gap-2 p-2">
					<WarningIcon size={20} class="shrink-0 text-red" />
					<div class="min-w-0">
						<p class="text-sm font-medium text-default">Could not open this folder</p>
						<p class="truncate text-xs text-dim">{errorMessage}</p>
					</div>
				</Card>
			{:else if isLoading}
				<div class="flex justify-center p-2">
					<LoadingSpinner />
				</div>
			{:else}
				<SectionHeader title={listingSummary()} />
				{#if actionError}
					<button
						type="button"
						onclick={() => (actionError = "")}
						class="flex items-center gap-2 rounded-lg bg-raised p-2 text-left"
					>
						<WarningIcon size={16} class="shrink-0 text-red" />
						<span class="min-w-0 flex-1 truncate text-xs text-dim">{actionError}</span>
					</button>
				{/if}
				{#if isFindOpen}
					<FindField
						names={visibleEntries.map((entry) => entry.name)}
						focusIndex={selection.focusIndex}
						onMatch={jumpToFound}
						onClose={closeFind}
					/>
				{/if}
				<div
					bind:this={listContainer}
					role="listbox"
					tabindex="0"
					aria-label="Directory contents"
					aria-multiselectable="true"
					aria-activedescendant={activeDescendant()}
					oncontextmenu={openBackgroundMenu}
					onpointerdown={startBand}
					onpointermove={updateBand}
					onpointerup={endBand}
					onscroll={trackScroll}
					class="min-h-0 flex-1 overflow-y-auto rounded-lg focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-line-faint focus-visible:outline-none"
				>
					{#if viewMode === "grid"}
						<EntryGrid
							entries={mountedEntries}
							firstIndex={mountedWindow.firstIndex}
							topPadding={mountedWindow.topPadding}
							bottomPadding={mountedWindow.bottomPadding}
							columns={activeColumns}
							{selectedPaths}
							{cutPaths}
							focusIndex={selection.focusIndex}
							{renamingPath}
							onSelect={selectEntry}
							onOpen={openEntry}
							onContextMenu={openContextMenu}
							onRenameCommit={commitRename}
							onRenameCancel={cancelRename}
							onMetrics={applyMetrics}
						/>
					{:else}
						<EntryRows
							entries={mountedEntries}
							firstIndex={mountedWindow.firstIndex}
							topPadding={mountedWindow.topPadding}
							bottomPadding={mountedWindow.bottomPadding}
							columns={activeColumns}
							{selectedPaths}
							{cutPaths}
							focusIndex={selection.focusIndex}
							{renamingPath}
							onSelect={selectEntry}
							onOpen={openEntry}
							onContextMenu={openContextMenu}
							onRenameCommit={commitRename}
							onRenameCancel={cancelRename}
							onMetrics={applyMetrics}
							order={sortOrder}
							onSort={sortByColumn}
						/>
					{/if}
				</div>
			{/if}
		</Card>

		{#if selectedEntries.length > 0}
			<PreviewPane entries={selectedEntries} />
		{/if}
	</div>
</div>

{#if band}
	<div
		class="pointer-events-none fixed z-30 rounded-sm border border-line-strong bg-hover/40"
		style="left: {band.left}px; top: {band.top}px; width: {band.width}px; height: {band.height}px"
	></div>
{/if}

{#if menu}
	<ContextMenu
		x={menu.x}
		y={menu.y}
		groups={menu.groups}
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
