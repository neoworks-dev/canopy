<script lang="ts">
	import { onMount } from "svelte";
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
		OpenPath,
		OpenWith,
		Places,
		ReadClipboardPaths,
		RenameEntry,
		SetDefaultApplication,
		SuggestName,
	} from "../wailsjs/go/main/FileService";
	import { IndexStatus, Search } from "../wailsjs/go/main/IndexService";
	import type { filesystem, proto } from "../wailsjs/go/models";
	import ColumnMenu from "./lib/ColumnMenu.svelte";
	import ConfirmDialog from "./lib/ConfirmDialog.svelte";
	import ConflictDialog from "./lib/ConflictDialog.svelte";
	import ContextMenu from "./lib/ContextMenu.svelte";
	import EntryGrid from "./lib/EntryGrid.svelte";
	import EntryRows from "./lib/EntryRows.svelte";
	import OpenWithDialog from "./lib/OpenWithDialog.svelte";
	import OperationProgress from "./lib/OperationProgress.svelte";
	import PathBar from "./lib/PathBar.svelte";
	import PlacesSidebar from "./lib/PlacesSidebar.svelte";
	import PreviewPane from "./lib/PreviewPane.svelte";
	import SearchField from "./lib/SearchField.svelte";
	import SearchResults from "./lib/SearchResults.svelte";
	import TrashPane from "./lib/TrashPane.svelte";
	import {
		backgroundMenuGroups,
		selectionMenuGroups,
		type ActionContext,
		type ActionDefinition,
		type ActionId,
	} from "./lib/actions";
	import { columnsFor, defaultColumnIds, type ColumnId } from "./lib/columns";
	import { entryElementId } from "./lib/entryList";
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
	import { targetIndexFor } from "./lib/movement";
	import { OperationQueue } from "./lib/operations.svelte";
	import {
		emptySelection,
		pruneSelection,
		selectAll,
		selectionAfterClick,
		selectionAfterJump,
		selectionFromIndexes,
		type Selection,
	} from "./lib/selection";
	import {
		emptyTypeAhead,
		isTypeAheadKey,
		typeAheadAfterKey,
		typeAheadMatch,
	} from "./lib/typeAhead";

	// Long enough that typing a word does not fire a query per keystroke, short
	// enough that results feel immediate.
	const searchDebounceMilliseconds = 150;
	const statusPollMilliseconds = 3000;

	type ViewMode = "rows" | "grid";

	type SearchOutcome = {
		hits: proto.Hit[];
		truncated: boolean;
		errorMessage: string;
	};

	type MenuState = {
		x: number;
		y: number;
		groups: ActionDefinition[][];
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
	let selection = $state<Selection>(emptySelection);
	let history = $state(emptyHistory);
	let isEditingPath = $state(false);
	let isTrashOpen = $state(false);
	let gridColumnCount = $state(1);
	let listContainer = $state<HTMLDivElement>();
	let searchInput = $state<HTMLInputElement>();

	let clipboardPaths = $state<string[]>([]);
	let isClipboardCut = $state(false);

	let menu = $state<MenuState | null>(null);
	let renamingPath = $state("");
	let isConfirmingDelete = $state(false);
	let openWithPath = $state("");
	let typeAhead = emptyTypeAhead;

	let band = $state<Rectangle | null>(null);
	let bandOrigin: Point = { x: 0, y: 0 };
	let bandKeep: string[] = [];

	let searchQuery = $state("");
	let searchHits = $state<proto.Hit[]>([]);
	let searchTruncated = $state(false);
	let searchErrorMessage = $state("");
	let isSearching = $state(false);
	let indexStatus = $state<proto.StatusResult | null>(null);

	// Guards against an early slow search — the first one also starts the daemon —
	// overwriting the results of a later, faster one.
	let latestSearchSequence = 0;

	const operations = new OperationQueue(() => reloadListing());

	const currentPath = $derived(pathOf(listing));
	const canGoUp = $derived(Boolean(listing?.parentPath));
	const visibleEntries = $derived.by(collectVisibleEntries);
	const visiblePaths = $derived(visibleEntries.map((entry) => entry.path));
	const selectedPaths = $derived(new Set(selection.paths));
	const selectedEntries = $derived(visibleEntries.filter((entry) => selectedPaths.has(entry.path)));
	const activeColumns = $derived(columnsFor(visibleColumnIds));
	const isSearchActive = $derived(searchQuery.trim().length > 0);
	const isIndexing = $derived(indexStatus?.state === "scanning");
	const actionContext = $derived(currentActionContext());
	const conflict = $derived(operations.conflict);
	// Only a cut marks entries as pending; a copy leaves them as they are.
	const cutPaths = $derived(new Set(pathsHeldByCut()));

	onMount(() => {
		navigateTo("");
		loadPlaces();
		return operations.listen();
	});

	$effect(() => {
		const query = searchQuery.trim();
		if (!query) {
			clearSearch();
			return;
		}
		selection = emptySelection;
		const timer = setTimeout(() => runSearch(query), searchDebounceMilliseconds);
		return () => clearTimeout(timer);
	});

	// Hiding dotfiles must not leave invisible entries selected.
	$effect(() => {
		selection = pruneSelection(selection, visiblePaths);
	});

	// Keyboard movement is useless if the entry it lands on is off screen.
	$effect(() => {
		revealEntry(selection.focusIndex);
	});

	// The listing is the default keyboard target, so arrows work without a click
	// first — but only while nothing else has claimed focus.
	$effect(() => {
		claimInitialFocus(listContainer);
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

	async function navigateTo(path: string) {
		isTrashOpen = false;
		isEditingPath = false;
		isLoading = true;
		errorMessage = "";
		selection = emptySelection;
		try {
			listing = await ListDirectory(path);
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
		try {
			listing = await ListDirectory(currentPath);
		} catch (error) {
			errorMessage = describeError(error);
		}
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

	async function runSearch(query: string) {
		const sequence = ++latestSearchSequence;
		isSearching = true;
		const outcome = await requestSearch(query);
		if (sequence !== latestSearchSequence) {
			return;
		}
		applySearchOutcome(outcome);
		isSearching = false;
	}

	async function requestSearch(query: string): Promise<SearchOutcome> {
		try {
			const result = await Search(query);
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

	function revealEntry(index: number) {
		if (index < 0 || !listContainer) {
			return;
		}
		const option = listContainer.querySelector(`#${entryElementId(index)}`);
		option?.scrollIntoView({ block: "nearest" });
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

	function currentActionContext(): ActionContext {
		return {
			selectionCount: selectedEntries.length,
			clipboardCount: clipboardPaths.length,
			canUndo: operations.undoLabel !== "",
		};
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
			listing = await ListDirectory(currentLocation(history));
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

	function showRows() {
		viewMode = "rows";
		gridColumnCount = 1;
	}

	function showGrid() {
		viewMode = "grid";
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

	function runAction(id: ActionId) {
		closeMenu();
		actionHandlers[id]();
	}

	const actionHandlers: Record<ActionId, () => void> = {
		open: openSelection,
		openWith: startOpenWith,
		cut: cutSelection,
		copy: copySelection,
		paste: pasteClipboard,
		rename: startRename,
		trash: trashSelection,
		deletePermanently: () => (isConfirmingDelete = true),
		newFolder: () => createEntry("folder"),
		newFile: () => createEntry("file"),
		undo: () => operations.undo(),
	};

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

	// A drag that starts on an entry is a click on that entry; only the space
	// around them opens a rubber band.
	function startBand(event: PointerEvent) {
		if (event.button !== 0 || startsOnEntry(event)) {
			return;
		}
		bandOrigin = { x: event.clientX, y: event.clientY };
		bandKeep = keptDuringBand(event);
		selection = bandSelection([]);
		band = rectangleBetween(bandOrigin, bandOrigin);
		listContainer?.setPointerCapture(event.pointerId);
	}

	function startsOnEntry(event: PointerEvent): boolean {
		if (!(event.target instanceof HTMLElement)) {
			return false;
		}
		return Boolean(event.target.closest('[role="option"]'));
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
		selection = bandSelection(indexesWithin(current, entryRectangles()));
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

	function entryRectangles(): Rectangle[] {
		if (!listContainer) {
			return [];
		}
		const options = listContainer.querySelectorAll('[role="option"]');
		return [...options].map((option) => option.getBoundingClientRect());
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (isTypingTarget(event.target)) {
			return;
		}
		if (applyGlobalShortcut(event)) {
			event.preventDefault();
		}
	}

	function isTypingTarget(target: EventTarget | null): boolean {
		if (!(target instanceof HTMLElement)) {
			return false;
		}
		return target.tagName === "INPUT" || target.isContentEditable;
	}

	function applyGlobalShortcut(event: KeyboardEvent): boolean {
		if (event.altKey) {
			return applyHistoryShortcut(event.key);
		}
		if (event.ctrlKey || event.metaKey) {
			return applyControlShortcut(event);
		}
		if (event.key === "Backspace") {
			goUp();
			return true;
		}
		return false;
	}

	function applyHistoryShortcut(key: string): boolean {
		if (key === "ArrowLeft") {
			goBack();
			return true;
		}
		if (key === "ArrowRight") {
			goForward();
			return true;
		}
		return false;
	}

	function applyControlShortcut(event: KeyboardEvent): boolean {
		if (event.key === "l") {
			isEditingPath = true;
			return true;
		}
		if (event.key === "f") {
			focusSearch();
			return true;
		}
		if (event.key === "h") {
			toggleHidden();
			return true;
		}
		if (event.key === "z") {
			operations.undo();
			return true;
		}
		return applyControlListShortcut(event);
	}

	function applyControlListShortcut(event: KeyboardEvent): boolean {
		if (event.key === "N" && event.shiftKey) {
			createEntry("folder");
			return true;
		}
		if (event.key === "a") {
			selection = selectAll(visiblePaths);
			return true;
		}
		return applyClipboardShortcut(event.key);
	}

	function applyClipboardShortcut(key: string): boolean {
		const handlers: Record<string, () => void> = {
			c: copySelection,
			x: cutSelection,
			v: pasteClipboard,
		};
		const handler = handlers[key];
		if (!handler) {
			return false;
		}
		handler();
		return true;
	}

	function handleListKeydown(event: KeyboardEvent) {
		if (event.ctrlKey || event.metaKey || event.altKey) {
			return;
		}
		if (applyListShortcut(event)) {
			event.preventDefault();
		}
	}

	function applyListShortcut(event: KeyboardEvent): boolean {
		if (event.key === "Escape") {
			selection = emptySelection;
			return true;
		}
		if (event.key === "Enter") {
			openSelection();
			return true;
		}
		if (event.key === "F2") {
			startRename();
			return true;
		}
		if (event.key === "Delete") {
			applyDeleteShortcut(event.shiftKey);
			return true;
		}
		return applyMovementShortcut(event);
	}

	function applyDeleteShortcut(permanent: boolean) {
		if (permanent) {
			isConfirmingDelete = true;
			return;
		}
		trashSelection();
	}

	function applyMovementShortcut(event: KeyboardEvent): boolean {
		const target = targetIndexFor(event.key, selection.focusIndex, movementContext());
		if (target >= 0) {
			selection = selectionAfterJump(selection, visiblePaths, target, event.shiftKey);
			return true;
		}
		return applyTypeAhead(event);
	}

	function movementContext() {
		return { total: visiblePaths.length, columns: gridColumnCount, pageSize: viewportPageSize() };
	}

	// One page is however many entries currently fit in the scroll area.
	function viewportPageSize(): number {
		const option = listContainer?.querySelector('[role="option"]');
		if (!option || !listContainer) {
			return 10;
		}
		const rows = Math.floor(listContainer.clientHeight / option.clientHeight);
		return Math.max(1, rows) * gridColumnCount;
	}

	function applyTypeAhead(event: KeyboardEvent): boolean {
		if (!isTypeAheadKey(event)) {
			return false;
		}
		typeAhead = typeAheadAfterKey(typeAhead, event.key, Date.now());
		const names = visibleEntries.map((entry) => entry.name);
		const match = typeAheadMatch(names, typeAhead.query, selection.focusIndex);
		if (match < 0) {
			return true;
		}
		selection = selectionAfterJump(selection, visiblePaths, match, false);
		return true;
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

		<SearchField bind:value={searchQuery} bind:input={searchInput} />

		<Button size="sm" variant={viewVariant("rows")} icon={ListIcon} onclick={showRows} />
		<Button size="sm" variant={viewVariant("grid")} icon={SquaresFourIcon} onclick={showGrid} />

		<ColumnMenu visibleIds={visibleColumnIds} onChange={setColumns} />

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
							<SearchResults hits={searchHits} truncated={searchTruncated} onOpen={openHit} />
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
				<div
					bind:this={listContainer}
					role="listbox"
					tabindex="0"
					aria-label="Directory contents"
					aria-multiselectable="true"
					aria-activedescendant={activeDescendant()}
					onkeydown={handleListKeydown}
					oncontextmenu={openBackgroundMenu}
					onpointerdown={startBand}
					onpointermove={updateBand}
					onpointerup={endBand}
					class="min-h-0 flex-1 overflow-y-auto rounded-lg focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-line-faint focus-visible:outline-none"
				>
					{#if viewMode === "grid"}
						<EntryGrid
							entries={visibleEntries}
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
							onColumnCountChange={(count) => (gridColumnCount = count)}
						/>
					{:else}
						<EntryRows
							entries={visibleEntries}
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
		context={actionContext}
		onSelect={runAction}
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
