<!--
	One pane of the window: a tab strip above whatever its tab is showing. A folder
	is a listing in the view the user chose (rows, tiles or Miller columns) with a
	rubber band for selecting, a find prompt, inline rename, and drag and drop; the
	other kinds of location are panes of their own. The App drives the listing
	through the methods this component exports.
-->
<script lang="ts">
	import { untrack, type Snippet } from "svelte";
	import {
		Button as PlainButton,
		DropTarget,
		Icon,
		Overlay,
		Text,
		View,
		type FileDragEvent,
		type Layout,
		type OutsideEvent,
		type PointerEvent,
	} from "@neoworks/svelte-gpui/ui";
	import { CreateFile, CreateFolder, GitReport, ListDirectory, RenameEntry, SuggestName } from "../backend/FileService";
	import { EventsOn } from "../backend/events";
	import type { filesystem, git } from "../backend/models";
	import { funnel, warning } from "../icons";
	import { isArchiveName } from "../lib/archive";
	import { columnsFor, type ColumnId } from "../lib/columns";
	import {
		dropOperation,
		pathsForDrag,
		type DragModifiers,
		type DropOperation,
	} from "../lib/dragDrop";
	import { describeError } from "../lib/errors";
	import { parentDirectoryOf } from "../lib/format";
	import {
		clampToBounds,
		emptyGridLayout,
		exceedsThreshold,
		indexAtPoint,
		indexesInGrid,
		rectangleBetween,
		toContentPoint,
		toViewportPoint,
		type GridLayout,
		type Point,
		type Rectangle,
	} from "../lib/marquee";
	import { nameMatches, nextMatchIndex, type FindDirection } from "../lib/finder";
	import {
		allMarked,
		cursorMoved,
		markToggled,
		marksInverted,
		visualMoved,
		visualStarted,
		type MarkState,
		type Visual,
		type VisualMode,
	} from "../lib/marks";
	import { targetIndexFor, type Movement } from "../lib/movement";
	import type { PaneStatus } from "../lib/paneStatus";
	import type { SavedSearch } from "../lib/savedSearches";
	import {
		emptySelection,
		pruneSelection,
		selectionAfterClick,
		selectionAfterJump,
		selectionAfterRelisting,
		selectionFromIndexes,
		type Selection,
	} from "../lib/selection";
	import type { ViewMode } from "../lib/settings";
	import type { SortKey, SortOrder } from "../lib/sorting";
	import { directoryPathOf, locationKey, type TabLocation } from "../lib/tabs";
	import { pageSize } from "../lib/virtualList";
	import { directoryChangedEvent } from "../lib/watch";
	import { colors, duration, radius, shadow, space, textSize } from "../theme";
	import ArchivePane from "./ArchivePane.svelte";
	import DuplicatesPane from "./DuplicatesPane.svelte";
	import EntryGrid from "./EntryGrid.svelte";
	import EntryRows from "./EntryRows.svelte";
	import FilterField from "./FilterField.svelte";
	import FindField from "./FindField.svelte";
	import MillerColumns from "./MillerColumns.svelte";
	import SearchPane from "./SearchPane.svelte";
	import TagPane from "./TagPane.svelte";
	import TrashPane from "./TrashPane.svelte";
	import Card from "./design/Card.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";

	type DraggedPaths = { paths: string[] };

	let {
		location,
		viewMode,
		visibleColumnIds,
		sortOrder,
		showHidden,
		cutPaths,
		savedSearches,
		onStatus,
		onNavigate,
		onActivate,
		onLaunch,
		onSelectionMenu,
		onBackgroundMenu,
		onSort,
		onChanged,
		onDropPaths,
		onExtractArchive,
		header,
		aside,
	}: {
		location: TabLocation;
		viewMode: ViewMode;
		visibleColumnIds: ColumnId[];
		sortOrder: SortOrder;
		showHidden: boolean;
		cutPaths: Set<string>;
		/** Looked up by name when this pane is showing one as a folder. */
		savedSearches: SavedSearch[];
		onStatus: (status: PaneStatus) => void;
		onNavigate: (location: TabLocation) => void;
		onActivate: () => void;
		onLaunch: (path: string) => void;
		/** The press that asked for it carries where it landed in the window. */
		onSelectionMenu: (event: PointerEvent) => void;
		onBackgroundMenu: (event: PointerEvent) => void;
		onSort: (key: SortKey) => void;
		/** Something in this pane wrote to disk, which the undo label follows. */
		onChanged: () => void;
		/** A drag landed here: hands the transfer to the operation engine. */
		onDropPaths: (sources: string[], destination: string, operation: DropOperation) => void;
		/** Unpacks the archive this pane is showing into a folder beside it. */
		onExtractArchive: (archivePath: string) => void;
		/** Rendered flush against the top edge of the pane, above the listing. */
		header?: Snippet;
		/** Rendered beside the listing, below the header, inside the same card. */
		aside?: Snippet;
	} = $props();

	// Raw: a listing is replaced whole, and deep proxies over thousands of entries cost memory.
	let listing = $state.raw<filesystem.Listing | null>(null);
	let errorMessage = $state("");
	// Failures of one action, reported above the listing instead of replacing it.
	let actionError = $state("");
	let isLoading = $state(false);
	let selection = $state<Selection>(emptySelection);
	let renamingPath = $state("");
	let isFindOpen = $state(false);
	// Whether the selection is marks the cursor has left behind rather than just
	// the entry under it.
	let isMarking = $state(false);
	let visual = $state.raw<Visual | null>(null);
	let isFilterOpen = $state(false);
	let filterQuery = $state("");
	let findQuery = $state("");
	let findDirection = $state<FindDirection>("forward");
	let loadedLocationKey = "";
	let gitReport = $state.raw<git.Report | null>(null);
	// Set while entries dragged out of this pane are still in flight, and while
	// files dragged in from another program are over it.
	let isDraggingEntries = $state(false);
	let isOutsideDragOver = $state(false);
	let modifiers = $state<DragModifiers>({ ctrlKey: false, shiftKey: false, metaKey: false });

	// Where the entries sit and how far they are scrolled, which a band, a right
	// press and the page keys are matched against.
	let gridLayout = $state<GridLayout>(emptyGridLayout);
	let scrollTop = $state(0);
	// Where the listing is in the window, and how big.
	let container = $state<Layout>({ x: 0, y: 0, width: 0, height: 0 });
	let listContainer = $state<View>();
	let rowsView = $state<EntryRows>();
	let gridView = $state<EntryGrid>();
	let columnsView = $state<MillerColumns>();

	// Guards against the report of a folder this pane has already left arriving
	// after the one it is showing now.
	let latestGitRequest = 0;

	// What the selection was before the find prompt started moving it, so
	// cancelling puts it back.
	let findOrigin: Selection = emptySelection;

	let band = $state<Rectangle | null>(null);
	// The drag start in content coordinates, so it stays on the entry it began on
	// while the listing scrolls; the pointer stays where it is on screen.
	let bandAnchor: Point = { x: 0, y: 0 };
	let bandPointer: Point = { x: 0, y: 0 };
	let bandKeep: string[] = [];

	const directoryPath = $derived(directoryPathOf(location));
	const visibleEntries = $derived.by(collectVisibleEntries);
	const visiblePaths = $derived(visibleEntries.map((entry) => entry.path));
	const selectedPaths = $derived(new Set(selection.paths));
	const selectedEntries = $derived(visibleEntries.filter((entry) => selectedPaths.has(entry.path)));
	const activeColumns = $derived(columnsFor(visibleColumnIds));
	const gitStatuses = $derived(reportedStatuses());
	const viewport = $derived({ scrollTop, height: Math.max(0, container.height - gridLayout.top) });

	$effect(() => {
		return EventsOn(directoryChangedEvent, applyExternalChange);
	});

	// The location and the order are what the listing is read from, so a change
	// to either re-reads it.
	$effect(() => {
		const wanted = location;
		const order = sortOrder;
		untrack(() => {
			forgetFilterOnNewLocation(wanted);
			load(wanted, order);
		});
	});

	// Hiding dotfiles must not leave invisible entries selected.
	$effect(() => {
		selection = pruneSelection(selection, visiblePaths);
		if (selection.paths.length === 0) {
			isMarking = false;
		}
	});

	// Keyboard movement is useless if the entry it lands on is off screen. Only
	// the focus is a reason to scroll: reacting to the scroll as well would drag
	// the focused entry back every time the user scrolled away from it.
	$effect(() => {
		const index = selection.focusIndex;
		untrack(() => revealEntry(index));
	});

	// The report is tracked so a change to the listing or the selection sends a
	// fresh one; delivering it is not, or reading the callback back would make
	// the chrome's own re-render the reason to report again.
	$effect(() => {
		const reported = currentStatus();
		untrack(() => onStatus(reported));
	});

	export function reload() {
		reloadListing();
	}

	export function focusList() {
		listContainer?.focus();
	}

	export function selectEverything() {
		visual = null;
		applyMarks(allMarked(markState(), visiblePaths));
	}

	export function invertSelection() {
		visual = null;
		applyMarks(marksInverted(markState(), visiblePaths));
	}

	/** Leaves visual mode, else clears the marks, else the filter, else the find. */
	export function escape() {
		if (visual) {
			visual = null;
			return;
		}
		if (isMarking) {
			setSelection(selectionAfterJump(emptySelection, visiblePaths, selection.focusIndex, false));
			return;
		}
		if (filterQuery !== "") {
			filterQuery = "";
			return;
		}
		findQuery = "";
	}

	/** Marks or unmarks the entry under the cursor, then moves to the next one. */
	export function toggleFocusedEntry() {
		visual = null;
		const toggled = markToggled(markState(), visiblePaths);
		applyMarks(cursorMoved(toggled, visiblePaths, toggled.selection.focusIndex + 1));
	}

	/** Starts sweeping a range at the cursor; leaves the sweep if one is under way. */
	export function toggleVisual(mode: VisualMode) {
		if (visual) {
			visual = null;
			return;
		}
		if (selection.focusIndex < 0) {
			return;
		}
		visual = visualStarted(markState(), mode);
		applyMarks(visualMoved(visual, visiblePaths, selection.focusIndex));
	}

	export function moveSelection(movement: Movement, extend: boolean) {
		const target = targetIndexFor(movement, selection.focusIndex, movementContext());
		if (target < 0) {
			return;
		}
		if (extend) {
			setSelection(selectionAfterJump(selection, visiblePaths, target, true), true);
			return;
		}
		moveCursorTo(target);
	}

	export function openFilter() {
		isFilterOpen = true;
	}

	export function findNext(direction: FindDirection) {
		const target = nextMatchIndex(
			visibleEntries.map((entry) => entry.name),
			findQuery,
			selection.focusIndex,
			direction,
			false,
		);
		if (target < 0) {
			return;
		}
		moveCursorTo(target);
	}

	export function openSelection() {
		selectedEntries.forEach(openEntry);
	}

	/** Goes into the folder under the cursor; files and archives are left alone. */
	export function enterFocusedFolder() {
		const entry = visibleEntries[selection.focusIndex];
		if (!entry || !entry.isDir) {
			return;
		}
		onNavigate({ kind: "directory", path: entry.path });
	}

	export function openFind(direction: FindDirection) {
		findOrigin = selection;
		findDirection = direction;
		isFindOpen = true;
	}

	export function startRename() {
		if (selectedEntries.length !== 1) {
			return;
		}
		renamingPath = selectedEntries[0].path;
	}

	/** Puts the focus on one entry, which is how a search hit is arrived at. */
	export function revealPath(path: string) {
		const index = visiblePaths.indexOf(path);
		if (index < 0) {
			return;
		}
		setSelection(selectionAfterJump(emptySelection, visiblePaths, index, false));
		focusList();
	}

	/**
	 * Creates an entry under a free name and hands it straight to the inline
	 * editor, so naming it is one uninterrupted gesture.
	 */
	export async function createEntry(kind: "folder" | "file") {
		actionError = "";
		try {
			const created = await createAndReveal(kind);
			setSelection(selectionAfterJump(emptySelection, [created], 0, false));
			renamingPath = created;
		} catch (error) {
			actionError = describeError(error);
		}
		onChanged();
	}

	/** Creates an entry under the given name and puts the cursor on it. */
	export async function createNamed(kind: "folder" | "file", name: string) {
		actionError = "";
		try {
			const created = await createOfKind(kind, name);
			await reloadListing();
			revealPath(created);
		} catch (error) {
			actionError = describeError(error);
		}
		onChanged();
	}

	export function dismissActionError() {
		actionError = "";
	}

	// Dragged entries are no longer what the user is working on, so the selection
	// goes with them. A cancelled drag clears it too.
	export function endDrag() {
		isDraggingEntries = false;
		setSelection(emptySelection);
	}

	function currentStatus(): PaneStatus {
		return {
			path: directoryPath,
			parentPath: parentPathOf(listing),
			entryCount: visibleEntries.length,
			hiddenCount: hiddenEntryCount(),
			selected: selectedEntries,
			isListActive: isListActive(),
			branch: reportedBranch(),
		};
	}

	function reportedStatuses(): Record<string, string> {
		if (!gitReport) {
			return {};
		}
		return gitReport.statuses;
	}

	function reportedBranch(): string {
		if (!gitReport?.isRepo) {
			return "";
		}
		return gitReport.repository.branch;
	}

	// The working tree is read after the listing rather than with it, so a folder
	// appears at once and picks up its marks a moment later.
	async function refreshGitReport(path: string) {
		const request = ++latestGitRequest;
		if (!path) {
			gitReport = null;
			return;
		}
		const reported = await requestGitReport(path);
		if (request !== latestGitRequest) {
			return;
		}
		gitReport = reported;
	}

	async function requestGitReport(path: string): Promise<git.Report | null> {
		try {
			return await GitReport(path);
		} catch {
			return null;
		}
	}

	function parentPathOf(current: filesystem.Listing | null): string {
		if (!current || location.kind !== "directory") {
			return "";
		}
		return current.parentPath;
	}

	function hiddenEntryCount(): number {
		if (!listing) {
			return 0;
		}
		return listing.entries.length - visibleEntries.length;
	}

	/** The listing is the pane on screen, rather than trash, tags or an error. */
	function isListActive(): boolean {
		if (location.kind !== "directory") {
			return false;
		}
		return errorMessage === "" && !isLoading;
	}

	async function load(wanted: TabLocation, order: SortOrder) {
		setSelection(emptySelection);
		errorMessage = "";
		listing = null;
		if (wanted.kind !== "directory") {
			return;
		}
		isLoading = true;
		try {
			listing = await ListDirectory(wanted.path, order);
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
		refreshGitReport(wanted.path);
	}

	/** Re-reads the directory in place, keeping the selection where it can. */
	async function reloadListing() {
		if (!directoryPath) {
			return;
		}
		const previousPaths = visiblePaths;
		try {
			listing = await ListDirectory(directoryPath, sortOrder);
		} catch (error) {
			errorMessage = describeError(error);
			return;
		}
		selection = selectionAfterRelisting(selection, previousPaths, visiblePaths);
		if (selection.paths.length === 0) {
			isMarking = false;
		}
		refreshGitReport(directoryPath);
	}

	// A change reported for a directory this pane is not showing belongs to
	// another pane.
	function applyExternalChange(path: string) {
		if (path !== directoryPath) {
			return;
		}
		reloadListing();
	}

	function collectVisibleEntries(): filesystem.Entry[] {
		if (!listing) {
			return [];
		}
		return listing.entries.filter(isShown);
	}

	function isShown(entry: filesystem.Entry): boolean {
		if (!showHidden && entry.name.startsWith(".")) {
			return false;
		}
		return nameMatches(entry.name, filterQuery);
	}

	// A filter or a find belongs to the folder it was typed in.
	function forgetFilterOnNewLocation(wanted: TabLocation) {
		const key = locationKey(wanted);
		if (key === loadedLocationKey) {
			return;
		}
		loadedLocationKey = key;
		filterQuery = "";
		findQuery = "";
		isFilterOpen = false;
		isFindOpen = false;
	}

	function markState(): MarkState {
		return { selection, isMarking };
	}

	function applyMarks(next: MarkState) {
		selection = next.selection;
		isMarking = next.isMarking;
	}

	function setSelection(next: Selection, marking = false) {
		selection = next;
		isMarking = marking;
		visual = null;
	}

	function moveCursorTo(index: number) {
		if (visual) {
			applyMarks(visualMoved(visual, visiblePaths, index));
			return;
		}
		applyMarks(cursorMoved(markState(), visiblePaths, index));
	}

	// A press selects the entry, and a second press in quick succession opens it.
	function pressEntry(index: number, event: PointerEvent) {
		onActivate();
		focusList();
		if (event.clicks >= 2) {
			openEntry(visibleEntries[index]);
			return;
		}
		const isAdding = event.ctrl || event.meta || event.shift;
		setSelection(
			selectionAfterClick(selection, visiblePaths, index, {
				toggle: event.ctrl || event.meta,
				range: event.shift,
			}),
			isAdding,
		);
	}

	// An archive opens as a pane of what it holds rather than in whatever
	// application the desktop would hand it to.
	function openEntry(entry: filesystem.Entry) {
		if (entry.isDir) {
			onNavigate({ kind: "directory", path: entry.path });
			return;
		}
		if (isArchiveName(entry.name)) {
			onNavigate({ kind: "archive", path: entry.path, inside: "" });
			return;
		}
		onLaunch(entry.path);
	}

	function revealEntry(index: number) {
		if (index < 0) {
			return;
		}
		if (viewMode === "columns") {
			columnsView?.scrollToIndex(index);
			return;
		}
		if (viewMode === "grid") {
			gridView?.scrollToIndex(index);
			return;
		}
		rowsView?.scrollToIndex(index);
	}

	function movementContext() {
		const metrics = {
			itemHeight: gridLayout.cellHeight,
			columns: gridLayout.columns,
			leadingHeight: gridLayout.top,
		};
		return {
			total: visiblePaths.length,
			columns: gridLayout.columns,
			pageSize: pageSize(metrics, viewport),
		};
	}

	function closeFind(accepted: boolean) {
		isFindOpen = false;
		if (!accepted) {
			selection = findOrigin;
			findQuery = "";
		}
		focusList();
	}

	function jumpToFound(index: number) {
		moveCursorTo(index);
	}

	function closeFilter(accepted: boolean) {
		isFilterOpen = false;
		if (!accepted) {
			filterQuery = "";
		}
		focusList();
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
		onChanged();
	}

	function cancelRename() {
		renamingPath = "";
	}

	async function createAndReveal(kind: "folder" | "file"): Promise<string> {
		const name = await SuggestName(directoryPath, defaultNameFor(kind));
		const created = await createOfKind(kind, name);
		await reloadListing();
		return created;
	}

	function createOfKind(kind: "folder" | "file", name: string): Promise<string> {
		if (kind === "folder") {
			return CreateFolder(directoryPath, name);
		}
		return CreateFile(directoryPath, name);
	}

	function defaultNameFor(kind: "folder" | "file"): string {
		if (kind === "folder") {
			return "New folder";
		}
		return "New file";
	}

	// Right-clicking outside the selection moves it, the way every file manager
	// behaves; right-clicking inside keeps the multi-selection intact.
	function openContextMenu(index: number, event: PointerEvent) {
		onActivate();
		focusList();
		selectForMenu(index);
		onSelectionMenu(event);
	}

	function selectForMenu(index: number) {
		if (selectedPaths.has(visiblePaths[index])) {
			return;
		}
		setSelection(selectionAfterJump(selection, visiblePaths, index, false));
	}

	// svelte-gpui: a Draggable takes every button that goes down on it, so an entry
	// never hears a right press itself. An overlay hears each press outside it with
	// where in the window it landed, which is matched against the entries' layout.
	function openContextMenuForPress(event: OutsideEvent) {
		if (!event.anchor || event.button !== 2) {
			return;
		}
		const point = { x: event.windowX - container.x, y: event.windowY - container.y };
		const index = indexAtPoint(point, scrollTop, gridLayout, visibleEntries.length);
		if (index < 0) {
			return;
		}
		openContextMenu(index, event);
	}

	// Only the space around the entries starts a band: an entry takes its own
	// presses, and so does everything above or beside them, such as the header.
	function pressBackground(event: PointerEvent) {
		onActivate();
		focusList();
		if (event.button === 2) {
			onBackgroundMenu(event);
			return;
		}
		if (event.button !== 0 || startsOutsideEntries(event)) {
			return;
		}
		beginBand(event);
	}

	function startsOutsideEntries(event: PointerEvent): boolean {
		return event.y < gridLayout.top || event.x < gridLayout.left;
	}

	function beginBand(event: PointerEvent) {
		bandPointer = pointerWithinList(event);
		bandAnchor = toContentPoint(bandPointer, listBounds(), scrollTop);
		bandKeep = keptDuringBand(event);
		setSelection(bandSelection([]), bandKeep.length > 0);
		band = rectangleBetween(bandPointer, bandPointer);
	}

	function keptDuringBand(event: PointerEvent): string[] {
		if (event.ctrl || event.meta) {
			return [...selection.paths];
		}
		return [];
	}

	function updateBand(event: PointerEvent) {
		if (!band) {
			return;
		}
		bandPointer = pointerWithinList(event);
		redrawBand();
	}

	function redrawBand() {
		const current = rectangleBetween(anchorOnScreen(), bandPointer);
		band = current;
		const swept = bandSelection(indexesInGrid(current, scrollTop, gridLayout, visibleEntries.length));
		setSelection(swept, swept.paths.length > 1);
	}

	function anchorOnScreen(): Point {
		const bounds = listBounds();
		return clampToBounds(toViewportPoint(bandAnchor, bounds, scrollTop), bounds);
	}

	// The pointer is captured, so a drag keeps reporting positions far outside the
	// listing; clamping them keeps the band inside the pane it selects in.
	function pointerWithinList(event: PointerEvent): Point {
		return clampToBounds({ x: event.x, y: event.y }, listBounds());
	}

	function listBounds(): Rectangle {
		return { left: 0, top: 0, width: container.width, height: container.height };
	}

	function endBand() {
		if (!band) {
			return;
		}
		clearSelectionAfterClick();
		band = null;
	}

	// A pointer that never really moved was a click on the background, which
	// clears the selection rather than selecting nothing in particular.
	function clearSelectionAfterClick() {
		if (band && exceedsThreshold(band)) {
			return;
		}
		setSelection(emptySelection);
	}

	function bandSelection(indexes: number[]): Selection {
		return selectionFromIndexes(visiblePaths, indexes, bandKeep);
	}

	function trackScroll(offset: number) {
		scrollTop = offset;
		if (!band) {
			return;
		}
		redrawBand();
	}

	function rememberModifiers(event: KeyboardEvent) {
		modifiers = { ctrlKey: event.ctrlKey, shiftKey: event.shiftKey, metaKey: event.metaKey };
	}

	/**
	 * Dragging an entry outside the selection takes the selection with it, the way
	 * a right-click does, so what moves is always what is highlighted.
	 */
	function startDrag(index: number) {
		onActivate();
		selectForDrag(index);
		isDraggingEntries = true;
	}

	function selectForDrag(index: number) {
		if (selectedPaths.has(visiblePaths[index])) {
			return;
		}
		setSelection(selectionAfterJump(selection, visiblePaths, index, false));
	}

	function dragPathsFor(entry: filesystem.Entry): string[] {
		return pathsForDrag(selection.paths, entry.path);
	}

	function acceptsPaths(data: DraggedPaths): boolean {
		return Array.isArray(data?.paths);
	}

	function dropOntoFolder(folder: filesystem.Entry, sources: string[]) {
		onDropPaths(sources, folder.path, dropOperation("internal", modifiers));
	}

	function dropIntoThisFolder(data: DraggedPaths) {
		onDropPaths(data.paths, directoryPath, dropOperation("internal", modifiers));
	}

	function dropFromOutside(event: FileDragEvent) {
		isOutsideDragOver = false;
		onDropPaths(event.paths, directoryPath, dropOperation("external", modifiers));
	}

	// Dropping entries back into the folder they came from does nothing, so the
	// listing only offers itself as a target to a drag from somewhere else.
	function wholeFolderRing(): { borderColor: string } {
		if (isDraggingEntries) {
			return { borderColor: "transparent" };
		}
		return { borderColor: colors.action };
	}

	function outsideDragRing(): string {
		if (isOutsideDragOver) {
			return colors.action;
		}
		return "transparent";
	}

	function savedSearchNamed(name: string): SavedSearch | undefined {
		return savedSearches.find((search) => search.name === name);
	}

	// A tagged file, one a saved search found, or one copy of several, is shown
	// where it lives, which means moving this pane to the folder holding it.
	function revealTaggedEntry(entry: filesystem.Entry) {
		revealPathInFolder(entry.path);
	}

	function revealPathInFolder(path: string) {
		onNavigate({ kind: "directory", path: parentDirectoryOf(path) });
	}
</script>

<svelte:window onkeydown={rememberModifiers} onkeyup={rememberModifiers} />

<Card
	surface="elevated"
	padding="none"
	style={{ flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", shadow: shadow.lg }}
>
	{@render header?.()}

	<View style={{ direction: "row", flex: 1, minHeight: 0 }}>
		<View style={{ flex: 1, minWidth: 0, minHeight: 0, gap: space[2], padding: space[2] }}>
			{#if location.kind === "trash"}
				<TrashPane onRestored={() => onChanged()} />
			{:else if location.kind === "search"}
				{@const search = savedSearchNamed(location.name)}
				{#if search}
					<SearchPane {search} onOpen={openEntry} onReveal={revealTaggedEntry} />
				{:else}
					<Text style={{ fontSize: textSize.sm, color: colors.dim }}>This search is no longer saved.</Text>
				{/if}
			{:else if location.kind === "archive"}
				{@const archive = location}
				<ArchivePane
					path={archive.path}
					inside={archive.inside}
					onDescend={(inside) => onNavigate({ kind: "archive", path: archive.path, inside })}
					onExtract={() => onExtractArchive(archive.path)}
				/>
			{:else if location.kind === "duplicates"}
				<DuplicatesPane
					path={location.path}
					onOpen={onLaunch}
					onReveal={revealPathInFolder}
					onChanged={() => onChanged()}
				/>
			{:else if location.kind === "tag"}
				<TagPane name={location.name} onOpen={openEntry} onReveal={revealTaggedEntry} />
			{:else if errorMessage}
				<Card
					surface="raised"
					padding="none"
					style={{ direction: "row", shrink: 0, align: "center", gap: space[2], padding: space[2] }}
				>
					<Icon svg={warning} size={20} color={colors.red} style={{ shrink: 0 }} />
					<View style={{ flex: 1, minWidth: 0 }}>
						<Text style={{ fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
							Could not open this folder
						</Text>
						<Text style={{ truncate: true, fontSize: textSize.xs, color: colors.dim }}>{errorMessage}</Text>
					</View>
				</Card>
			{:else if isLoading}
				<View style={{ align: "center", padding: space[2] }}><LoadingSpinner /></View>
			{:else}
				{#if actionError}
					<PlainButton
						style={{
							direction: "row",
							shrink: 0,
							align: "center",
							gap: space[2],
							padding: space[2],
							radius: radius.lg,
							background: colors.raised,
						}}
						onpress={dismissActionError}
					>
						<Icon svg={warning} size={16} color={colors.red} style={{ shrink: 0 }} />
						<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.xs, color: colors.dim }}>
							{actionError}
						</Text>
					</PlainButton>
				{/if}
				{#if isFilterOpen}
					<FilterField bind:query={filterQuery} onClose={closeFilter} />
				{/if}
				{#if filterQuery !== "" && !isFilterOpen}
					<View style={{ direction: "row", shrink: 0, align: "center", gap: space[2], paddingX: space[1] }}>
						<Icon svg={funnel} size={12} color={colors.faint} />
						<Text style={{ minWidth: 0, truncate: true, fontSize: textSize.xs, color: colors.muted }}>
							{filterQuery}
						</Text>
						<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>Esc to clear</Text>
					</View>
				{/if}
				{#if isFindOpen}
					<FindField
						bind:query={findQuery}
						names={visibleEntries.map((entry) => entry.name)}
						focusIndex={selection.focusIndex}
						direction={findDirection}
						onMatch={jumpToFound}
						onClose={closeFind}
					/>
				{/if}
				<DropTarget
					accept={acceptsPaths}
					ondrop={({ data }) => dropIntoThisFolder(data)}
					style={{ flex: 1, minHeight: 0, radius: radius.lg, borderWidth: 1, borderColor: "transparent" }}
					over={wholeFolderRing()}
				>
					<View
						bind:this={listContainer}
						focusable
						style={{
							flex: 1,
							minHeight: 0,
							radius: radius.lg,
							overflow: "hidden",
							borderWidth: 1,
							borderColor: outsideDragRing(),
							transition: duration.fast,
						}}
						focus={{ borderColor: colors.lineFaint }}
						onlayout={(layout: Layout) => (container = layout)}
						onfocus={onActivate}
						ondown={pressBackground}
						onmove={updateBand}
						onup={endBand}
						ondragenter={() => (isOutsideDragOver = true)}
						ondragover={() => (isOutsideDragOver = true)}
						ondragleave={() => (isOutsideDragOver = false)}
						ondrop={dropFromOutside}
					>
						{#if viewMode === "columns"}
							<MillerColumns
								bind:this={columnsView}
								currentPath={directoryPath}
								entries={visibleEntries}
								{selectedPaths}
								focusIndex={selection.focusIndex}
								{sortOrder}
								{showHidden}
								onPress={pressEntry}
								onOpen={openEntry}
								{dragPathsFor}
								onDragStart={startDrag}
								onDragEnd={endDrag}
								onDropOnFolder={dropOntoFolder}
								onLayout={(layout) => (gridLayout = layout)}
								onScroll={trackScroll}
								onNavigate={(path) => onNavigate({ kind: "directory", path })}
							/>
						{:else if viewMode === "grid"}
							<EntryGrid
								bind:this={gridView}
								entries={visibleEntries}
								columns={activeColumns}
								{selectedPaths}
								{cutPaths}
								{gitStatuses}
								focusIndex={selection.focusIndex}
								{renamingPath}
								onPress={pressEntry}
								{dragPathsFor}
								onDragStart={startDrag}
								onDragEnd={endDrag}
								onDropOnFolder={dropOntoFolder}
								onRenameCommit={commitRename}
								onRenameCancel={cancelRename}
								onLayout={(layout) => (gridLayout = layout)}
								onScroll={trackScroll}
							/>
						{:else}
							<EntryRows
								bind:this={rowsView}
								entries={visibleEntries}
								columns={activeColumns}
								{selectedPaths}
								{cutPaths}
								{gitStatuses}
								focusIndex={selection.focusIndex}
								{renamingPath}
								onPress={pressEntry}
								{dragPathsFor}
								onDragStart={startDrag}
								onDragEnd={endDrag}
								onDropOnFolder={dropOntoFolder}
								onRenameCommit={commitRename}
								onRenameCancel={cancelRename}
								onLayout={(layout) => (gridLayout = layout)}
								onScroll={trackScroll}
								order={sortOrder}
								{onSort}
							/>
						{/if}

						{#if band}
							<View
								style={{
									position: "absolute",
									left: band.left,
									top: band.top,
									width: band.width,
									height: band.height,
									radius: radius.xs,
									borderWidth: 1,
									borderColor: colors.lineStrong,
									background: "rgba(46, 46, 49, 0.4)",
								}}
							/>
						{/if}

						<Overlay passthrough onoutside={openContextMenuForPress}>
							<View style={{ size: 0 }} />
						</Overlay>
					</View>
				</DropTarget>
			{/if}
		</View>
		{@render aside?.()}
	</View>
</Card>
