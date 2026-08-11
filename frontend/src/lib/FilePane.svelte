<script lang="ts">
	import { untrack, type Snippet } from "svelte";
	import { Card, LoadingSpinner } from "@neoworks-dev/ui";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import {
		CreateFile,
		CreateFolder,
		GitReport,
		ListDirectory,
		RenameEntry,
		SuggestName,
	} from "../../wailsjs/go/main/FileService";
	import { EventsOn } from "../../wailsjs/runtime/runtime";
	import type { filesystem, git } from "../../wailsjs/go/models";
	import EntryGrid from "./EntryGrid.svelte";
	import EntryRows from "./EntryRows.svelte";
	import FindField from "./FindField.svelte";
	import TagPane from "./TagPane.svelte";
	import TrashPane from "./TrashPane.svelte";
	import { columnsFor, type ColumnId } from "./columns";
	import {
		dragSourceOf,
		dropDestination,
		dropOperation,
		internalDragType,
		pathsForDrag,
		readDraggedPaths,
		toUriList,
		uriListType,
		type DropOperation,
		type HoveredEntry,
	} from "./dragDrop";
	import { entryElementId, indexOfOption } from "./entryList";
	import { describeError } from "./errors";
	import { parentDirectoryOf } from "./format";
	import {
		clampToBounds,
		exceedsThreshold,
		indexesWithin,
		rectangleBetween,
		toContentPoint,
		toViewportPoint,
		type Point,
		type Rectangle,
	} from "./marquee";
	import { targetIndexFor, type Movement } from "./movement";
	import {
		emptySelection,
		pruneSelection,
		selectAll,
		selectionAfterClick,
		selectionAfterJump,
		selectionAfterRelisting,
		selectionFromIndexes,
		type Selection,
	} from "./selection";
	import type { ViewMode } from "./settings";
	import type { SortKey, SortOrder } from "./sorting";
	import type { PaneStatus } from "./paneStatus";
	import { directoryPathOf, type TabLocation } from "./tabs";
	import {
		listWindow,
		pageSize,
		sameMetrics,
		scrollTopRevealing,
		unmeasuredMetrics,
		type ListMetrics,
		type Viewport,
	} from "./virtualList";
	import { directoryChangedEvent } from "./watch";

	let {
		location,
		viewMode,
		visibleColumnIds,
		sortOrder,
		showHidden,
		cutPaths,
		onStatus,
		onNavigate,
		onActivate,
		onLaunch,
		onSelectionMenu,
		onBackgroundMenu,
		onSort,
		onChanged,
		onDropPaths,
		header,
	}: {
		location: TabLocation;
		viewMode: ViewMode;
		visibleColumnIds: ColumnId[];
		sortOrder: SortOrder;
		showHidden: boolean;
		cutPaths: Set<string>;
		onStatus: (status: PaneStatus) => void;
		onNavigate: (location: TabLocation) => void;
		onActivate: () => void;
		onLaunch: (path: string) => void;
		onSelectionMenu: (event: MouseEvent) => void;
		onBackgroundMenu: (event: MouseEvent) => void;
		onSort: (key: SortKey) => void;
		/** Something in this pane wrote to disk, which the undo label follows. */
		onChanged: () => void;
		/** A drag landed here: hands the transfer to the operation engine. */
		onDropPaths: (sources: string[], destination: string, operation: DropOperation) => void;
		/** Rendered flush against the top edge of the pane, above the listing. */
		header?: Snippet;
	} = $props();

	let listing = $state<filesystem.Listing | null>(null);
	let errorMessage = $state("");
	// Failures of one action, reported above the listing instead of replacing it.
	let actionError = $state("");
	let isLoading = $state(false);
	let selection = $state<Selection>(emptySelection);
	let listMetrics = $state<ListMetrics>(unmeasuredMetrics);
	let viewport = $state<Viewport>({ scrollTop: 0, height: 0 });
	let listContainer = $state<HTMLDivElement>();
	let renamingPath = $state("");
	let isFindOpen = $state(false);
	let gitReport = $state<git.Report | null>(null);
	// The folder a drag is hovering over, and whether it is over the pane at all.
	let dropTargetPath = $state("");
	let isDragOver = $state(false);
	// Set while entries dragged out of this pane are still in flight.
	let isDraggingEntries = $state(false);

	// Guards against the report of a folder this pane has already left arriving
	// after the one it is showing now.
	let latestGitRequest = 0;

	// What the selection was before the find prompt started moving it, so
	// cancelling puts it back.
	let findOrigin: Selection = emptySelection;

	let band = $state<Rectangle | null>(null);
	// The drag start in content coordinates, so it stays on the entry it began on
	// while the listing scrolls; the pointer stays in viewport coordinates.
	let bandAnchor: Point = { x: 0, y: 0 };
	let bandPointer: Point = { x: 0, y: 0 };
	let bandKeep: string[] = [];

	const directoryPath = $derived(directoryPathOf(location));
	const visibleEntries = $derived.by(collectVisibleEntries);
	const visiblePaths = $derived(visibleEntries.map((entry) => entry.path));
	const selectedPaths = $derived(new Set(selection.paths));
	const selectedEntries = $derived(
		visibleEntries.filter((entry) => selectedPaths.has(entry.path)),
	);
	const activeColumns = $derived(columnsFor(visibleColumnIds));
	const gitStatuses = $derived(reportedStatuses());
	// Only the entries around the viewport are mounted, so a directory of a
	// hundred thousand builds a screenful of DOM rather than all of it.
	const mountedWindow = $derived(listWindow(listMetrics, visibleEntries.length, viewport));
	const mountedEntries = $derived(
		visibleEntries.slice(mountedWindow.firstIndex, mountedWindow.endIndex),
	);

	$effect(() => {
		return EventsOn(directoryChangedEvent, applyExternalChange);
	});

	// The location and the order are what the listing is read from, so a change
	// to either re-reads it.
	$effect(() => {
		const wanted = location;
		const order = sortOrder;
		untrack(() => load(wanted, order));
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

	// The window depends on how much of the pane fits, which changes with the
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
		selection = selectAll(visiblePaths);
	}

	export function clearSelection() {
		selection = emptySelection;
	}

	export function toggleFocusedEntry() {
		if (selection.focusIndex < 0) {
			return;
		}
		selection = selectionAfterClick(selection, visiblePaths, selection.focusIndex, {
			toggle: true,
			range: false,
		});
	}

	export function moveSelection(movement: Movement, extend: boolean) {
		const target = targetIndexFor(movement, selection.focusIndex, movementContext());
		if (target < 0) {
			return;
		}
		selection = selectionAfterJump(selection, visiblePaths, target, extend);
	}

	export function openSelection() {
		selectedEntries.forEach(openEntry);
	}

	export function openFind() {
		findOrigin = selection;
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
		selection = selectionAfterJump(emptySelection, visiblePaths, index, false);
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
			selection = selectionAfterJump(emptySelection, [created], 0, false);
			renamingPath = created;
		} catch (error) {
			actionError = describeError(error);
		}
		onChanged();
	}

	export function dismissActionError() {
		actionError = "";
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
			isPointerDragging: band !== null || isDraggingEntries,
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
		selection = emptySelection;
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
		if (showHidden) {
			return listing.entries;
		}
		return listing.entries.filter((entry) => !entry.name.startsWith("."));
	}

	function selectEntry(index: number, event: MouseEvent) {
		onActivate();
		selection = selectionAfterClick(selection, visiblePaths, index, {
			toggle: event.ctrlKey || event.metaKey,
			range: event.shiftKey,
		});
	}

	function openEntry(entry: filesystem.Entry) {
		if (entry.isDir) {
			onNavigate({ kind: "directory", path: entry.path });
			return;
		}
		onLaunch(entry.path);
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
		viewport = {
			scrollTop: event.currentTarget.scrollTop,
			height: event.currentTarget.clientHeight,
		};
		if (!band) {
			return;
		}
		redrawBand();
	}

	function measureViewport(container: HTMLDivElement) {
		viewport = { scrollTop: container.scrollTop, height: container.clientHeight };
	}

	function activeDescendant(): string | undefined {
		if (selection.focusIndex < 0) {
			return undefined;
		}
		return entryElementId(selection.focusIndex);
	}

	function movementContext() {
		return {
			total: visiblePaths.length,
			columns: listMetrics.columns,
			pageSize: pageSize(listMetrics, viewport),
		};
	}

	function closeFind(accepted: boolean) {
		isFindOpen = false;
		if (!accepted) {
			selection = findOrigin;
		}
		focusList();
	}

	function jumpToFound(index: number) {
		selection = selectionAfterJump(selection, visiblePaths, index, false);
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
	function openContextMenu(index: number, event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();
		onActivate();
		selectForMenu(index);
		onSelectionMenu(event);
	}

	function selectForMenu(index: number) {
		if (selectedPaths.has(visiblePaths[index])) {
			return;
		}
		selection = selectionAfterJump(selection, visiblePaths, index, false);
	}

	function openEmptyAreaMenu(event: MouseEvent) {
		event.preventDefault();
		onActivate();
		onBackgroundMenu(event);
	}

	// A drag that starts on an entry is a click on that entry, and one that starts
	// on a control belongs to that control; only the space around them opens a
	// rubber band. Capturing the pointer here would otherwise redirect the
	// pointerup, and the control would never see a click at all.
	function startBand(event: PointerEvent) {
		onActivate();
		if (event.button !== 0 || startsOnInteractive(event)) {
			return;
		}
		bandPointer = pointerWithinList(event);
		bandAnchor = contentPointOf(bandPointer);
		bandKeep = keptDuringBand(event);
		selection = bandSelection([]);
		band = rectangleBetween(bandPointer, bandPointer);
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
		bandPointer = pointerWithinList(event);
		redrawBand();
	}

	function redrawBand() {
		const current = rectangleBetween(anchorOnScreen(), bandPointer);
		band = current;
		selection = bandSelection(indexesCoveredBy(current));
	}

	function anchorOnScreen(): Point {
		if (!listContainer) {
			return bandAnchor;
		}
		const bounds = listContainer.getBoundingClientRect();
		const onScreen = toViewportPoint(bandAnchor, bounds, listContainer.scrollTop);
		return clampToBounds(onScreen, bounds);
	}

	function contentPointOf(point: Point): Point {
		if (!listContainer) {
			return point;
		}
		const bounds = listContainer.getBoundingClientRect();
		return toContentPoint(point, bounds, listContainer.scrollTop);
	}

	// The pointer is captured, so a drag keeps reporting positions far outside the
	// listing; clamping them keeps the band inside the pane it selects in.
	function pointerWithinList(event: PointerEvent): Point {
		const point = { x: event.clientX, y: event.clientY };
		if (!listContainer) {
			return point;
		}
		const bounds = listContainer.getBoundingClientRect();
		return clampToBounds(point, bounds);
	}

	// A band can only cover what is on screen, so hit testing runs over the
	// mounted entries and reads back the index each one carries.
	function indexesCoveredBy(current: Rectangle): number[] {
		const options = mountedOptions();
		const covered = indexesWithin(
			current,
			options.map((option) => option.getBoundingClientRect()),
		);
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
	 * Dragging an entry outside the selection takes the selection with it, the way
	 * a right-click does, so what moves is always what is highlighted.
	 */
	function startDrag(index: number, event: DragEvent) {
		const entry = visibleEntries[index];
		if (!entry || !event.dataTransfer) {
			return;
		}
		selectForDrag(index);
		isDraggingEntries = true;
		writeDragPayload(event.dataTransfer, pathsForDrag(selection.paths, entry.path));
	}

	// Entries that landed somewhere are no longer what the user is working on, so
	// the selection goes with them and the details pane stays shut. A drag that
	// was cancelled leaves everything as it was.
	function endDrag(event: DragEvent) {
		isDraggingEntries = false;
		clearDragFeedback();
		if (event.dataTransfer?.dropEffect === "none") {
			return;
		}
		selection = emptySelection;
	}

	function selectForDrag(index: number) {
		if (selectedPaths.has(visiblePaths[index])) {
			return;
		}
		selection = selectionAfterJump(selection, visiblePaths, index, false);
	}

	// The URI list is what another application reads; the explorer's own type is
	// what tells a drop that it came from here rather than from outside.
	function writeDragPayload(transfer: DataTransfer, paths: string[]) {
		transfer.effectAllowed = "copyMove";
		transfer.setData(internalDragType, JSON.stringify(paths));
		transfer.setData(uriListType, toUriList(paths));
		transfer.setData("text/plain", paths.join("\n"));
	}

	// One handler on the listing rather than one per row: the rows are virtualized,
	// so the element under the pointer is found by hit testing instead.
	function trackDragOver(event: DragEvent) {
		if (!event.dataTransfer) {
			return;
		}
		event.preventDefault();
		isDragOver = true;
		dropTargetPath = folderPathUnder(event);
		event.dataTransfer.dropEffect = dropOperation(dragSourceOf(event.dataTransfer), event);
	}

	function folderPathUnder(event: DragEvent): string {
		const hovered = hoveredEntry(event);
		if (!hovered || !hovered.isDir) {
			return "";
		}
		return hovered.path;
	}

	function hoveredEntry(event: DragEvent): HoveredEntry | null {
		const option = optionUnder(event.target);
		if (!option) {
			return null;
		}
		const entry = visibleEntries[indexOfOption(option)];
		if (!entry) {
			return null;
		}
		return { path: entry.path, isDir: entry.isDir };
	}

	function optionUnder(target: EventTarget | null): Element | null {
		if (!(target instanceof Element)) {
			return null;
		}
		return target.closest('[role="option"]');
	}

	function applyDrop(event: DragEvent) {
		event.preventDefault();
		const transfer = event.dataTransfer;
		clearDragFeedback();
		if (!transfer) {
			return;
		}
		startDroppedOperation(transfer, event);
	}

	function startDroppedOperation(transfer: DataTransfer, event: DragEvent) {
		const sources = readDraggedPaths(transfer);
		const destination = dropDestination(hoveredEntry(event), directoryPath);
		onDropPaths(sources, destination, dropOperation(dragSourceOf(transfer), event));
	}

	function clearDragFeedback() {
		isDragOver = false;
		dropTargetPath = "";
	}

	// A tagged file is shown where it lives, which means moving this pane to the
	// folder holding it.
	function revealTaggedEntry(entry: filesystem.Entry) {
		onNavigate({ kind: "directory", path: parentDirectoryOf(entry.path) });
	}
</script>

<Card
	surface="elevated"
	padding="none"
	class="flex min-w-0 flex-1 flex-col overflow-hidden shadow-lg"
>
	{@render header?.()}

	<div class="flex min-h-0 flex-1 flex-col gap-2 p-2">
		{#if location.kind === "trash"}
			<div class="flex min-h-0 flex-col gap-2 overflow-y-auto">
				<TrashPane onRestored={() => onChanged()} />
			</div>
		{:else if location.kind === "tag"}
			<div class="flex min-h-0 flex-col gap-2 overflow-y-auto">
				<TagPane name={location.name} onOpen={openEntry} onReveal={revealTaggedEntry} />
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
			{#if actionError}
				<button
					type="button"
					onclick={dismissActionError}
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
				oncontextmenu={openEmptyAreaMenu}
				onpointerdown={startBand}
				onpointermove={updateBand}
				onpointerup={endBand}
				ondragover={trackDragOver}
				ondragleave={clearDragFeedback}
				ondragend={endDrag}
				ondrop={applyDrop}
				onfocus={onActivate}
				onscroll={trackScroll}
				class="min-h-0 flex-1 overflow-y-auto rounded-lg focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-line-faint focus-visible:outline-none"
				class:ring-1={isDragOver && dropTargetPath === ""}
				class:ring-inset={isDragOver && dropTargetPath === ""}
				class:ring-action={isDragOver && dropTargetPath === ""}
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
						{gitStatuses}
						focusIndex={selection.focusIndex}
						{renamingPath}
						{dropTargetPath}
						onSelect={selectEntry}
						onOpen={openEntry}
						onContextMenu={openContextMenu}
						onDragStart={startDrag}
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
						{gitStatuses}
						focusIndex={selection.focusIndex}
						{renamingPath}
						{dropTargetPath}
						onSelect={selectEntry}
						onOpen={openEntry}
						onContextMenu={openContextMenu}
						onDragStart={startDrag}
						onRenameCommit={commitRename}
						onRenameCancel={cancelRename}
						onMetrics={applyMetrics}
						order={sortOrder}
						{onSort}
					/>
				{/if}
			</div>
		{/if}
	</div>
</Card>

{#if band}
	<div
		class="pointer-events-none fixed z-30 rounded-sm border border-line-strong bg-hover/40"
		style="left: {band.left}px; top: {band.top}px; width: {band.width}px; height: {band.height}px"
	></div>
{/if}
