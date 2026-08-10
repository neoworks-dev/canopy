<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Card, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import ArrowUpIcon from "phosphor-svelte/lib/ArrowUpIcon";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import EyeIcon from "phosphor-svelte/lib/EyeIcon";
	import EyeSlashIcon from "phosphor-svelte/lib/EyeSlashIcon";
	import ListIcon from "phosphor-svelte/lib/ListIcon";
	import SquaresFourIcon from "phosphor-svelte/lib/SquaresFourIcon";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import { ListDirectory, Places } from "../wailsjs/go/main/FileService";
	import { IndexStatus, Search } from "../wailsjs/go/main/IndexService";
	import type { filesystem, proto } from "../wailsjs/go/models";
	import ColumnMenu from "./lib/ColumnMenu.svelte";
	import EntryGrid from "./lib/EntryGrid.svelte";
	import EntryRows from "./lib/EntryRows.svelte";
	import PathBreadcrumb from "./lib/PathBreadcrumb.svelte";
	import PlacesSidebar from "./lib/PlacesSidebar.svelte";
	import PreviewPane from "./lib/PreviewPane.svelte";
	import SearchField from "./lib/SearchField.svelte";
	import SearchResults from "./lib/SearchResults.svelte";
	import { columnsFor, defaultColumnIds, type ColumnId } from "./lib/columns";
	import { describeError } from "./lib/errors";
	import { parentDirectoryOf } from "./lib/format";
	import {
		emptySelection,
		pruneSelection,
		selectAll,
		selectionAfterClick,
		selectionAfterMove,
		type Selection,
	} from "./lib/selection";

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

	let listing = $state<filesystem.Listing | null>(null);
	let places = $state<filesystem.Place[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(false);
	let showHidden = $state(false);
	let viewMode = $state<ViewMode>("rows");
	let visibleColumnIds = $state<ColumnId[]>(defaultColumnIds);
	let selection = $state<Selection>(emptySelection);

	let searchQuery = $state("");
	let searchHits = $state<proto.Hit[]>([]);
	let searchTruncated = $state(false);
	let searchErrorMessage = $state("");
	let isSearching = $state(false);
	let indexStatus = $state<proto.StatusResult | null>(null);

	// Guards against an early slow search — the first one also starts the daemon —
	// overwriting the results of a later, faster one.
	let latestSearchSequence = 0;

	const currentPath = $derived(pathOf(listing));
	const canGoUp = $derived(Boolean(listing?.parentPath));
	const visibleEntries = $derived.by(collectVisibleEntries);
	const visiblePaths = $derived(visibleEntries.map((entry) => entry.path));
	const selectedPaths = $derived(new Set(selection.paths));
	const selectedEntries = $derived(visibleEntries.filter((entry) => selectedPaths.has(entry.path)));
	const activeColumns = $derived(columnsFor(visibleColumnIds));
	const isSearchActive = $derived(searchQuery.trim().length > 0);
	const isIndexing = $derived(indexStatus?.state === "scanning");

	onMount(() => {
		navigateTo("");
		loadPlaces();
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
		isLoading = true;
		errorMessage = "";
		selection = emptySelection;
		try {
			listing = await ListDirectory(path);
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
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

	// Opening a file means revealing it, since the explorer cannot launch it yet.
	function openHit(hit: proto.Hit) {
		searchQuery = "";
		if (hit.isDir) {
			navigateTo(hit.path);
			return;
		}
		navigateTo(parentDirectoryOf(hit.path));
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
		if (!entry.isDir) {
			return;
		}
		navigateTo(entry.path);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (isTypingTarget(event.target)) {
			return;
		}
		if (applyShortcut(event)) {
			event.preventDefault();
		}
	}

	function isTypingTarget(target: EventTarget | null): boolean {
		if (!(target instanceof HTMLElement)) {
			return false;
		}
		return target.tagName === "INPUT" || target.isContentEditable;
	}

	function applyShortcut(event: KeyboardEvent): boolean {
		if (isSearchActive) {
			return false;
		}
		if ((event.ctrlKey || event.metaKey) && event.key === "a") {
			selection = selectAll(visiblePaths);
			return true;
		}
		if (event.key === "Escape") {
			selection = emptySelection;
			return true;
		}
		if (event.key === "Enter") {
			return openSelection();
		}
		return moveSelection(event);
	}

	function moveSelection(event: KeyboardEvent): boolean {
		const step = arrowStep(event.key);
		if (step === 0) {
			return false;
		}
		selection = selectionAfterMove(selection, visiblePaths, step, event.shiftKey);
		return true;
	}

	function arrowStep(key: string): number {
		if (key === "ArrowDown") {
			return 1;
		}
		if (key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

	function openSelection(): boolean {
		if (selectedEntries.length !== 1) {
			return false;
		}
		const entry = selectedEntries[0];
		if (!entry.isDir) {
			return false;
		}
		navigateTo(entry.path);
		return true;
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

	function openPlace(path: string) {
		searchQuery = "";
		navigateTo(path);
	}

	function toggleHidden() {
		showHidden = !showHidden;
	}

	function setColumns(ids: ColumnId[]) {
		visibleColumnIds = ids;
	}

	function showRows() {
		viewMode = "rows";
	}

	function showGrid() {
		viewMode = "grid";
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
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-full flex-col gap-2 bg-canvas p-2">
	<Card surface="elevated" padding="none" class="flex shrink-0 items-center gap-2 p-2 shadow-lg">
		<Button size="sm" variant="ghost" icon={ArrowUpIcon} disabled={!canGoUp} onclick={goUp} />
		<Button size="sm" variant="ghost" icon={HouseIcon} onclick={goHome} />

		<PathBreadcrumb path={currentPath} onNavigate={navigateTo} />

		<SearchField bind:value={searchQuery} />

		<Button size="sm" variant={viewVariant("rows")} icon={ListIcon} onclick={showRows} />
		<Button size="sm" variant={viewVariant("grid")} icon={SquaresFourIcon} onclick={showGrid} />

		<ColumnMenu visibleIds={visibleColumnIds} onChange={setColumns} />

		<Button size="sm" variant="ghost" icon={hiddenToggleIcon()} onclick={toggleHidden} />
	</Card>

	<div class="flex min-h-0 flex-1 gap-2">
		{#if places.length > 0}
			<PlacesSidebar {places} {currentPath} onNavigate={openPlace} />
		{/if}

		<Card
			surface="elevated"
			padding="none"
			class="flex min-w-0 flex-1 flex-col gap-2 overflow-y-auto p-2 shadow-lg"
		>
			{#if isSearchActive}
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
				{#if viewMode === "grid"}
					<EntryGrid
						entries={visibleEntries}
						columns={activeColumns}
						{selectedPaths}
						onSelect={selectEntry}
						onOpen={openEntry}
					/>
				{:else}
					<EntryRows
						entries={visibleEntries}
						columns={activeColumns}
						{selectedPaths}
						onSelect={selectEntry}
						onOpen={openEntry}
					/>
				{/if}
			{/if}
		</Card>

		{#if selectedEntries.length > 0}
			<PreviewPane entries={selectedEntries} />
		{/if}
	</div>
</div>
