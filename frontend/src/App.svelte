<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Card, ListRow, SectionHeader, LoadingSpinner } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import ArrowUpIcon from "phosphor-svelte/lib/ArrowUpIcon";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import EyeIcon from "phosphor-svelte/lib/EyeIcon";
	import EyeSlashIcon from "phosphor-svelte/lib/EyeSlashIcon";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import { ListDirectory } from "../wailsjs/go/main/FileService";
	import { IndexStatus, Search } from "../wailsjs/go/main/IndexService";
	import type { filesystem, proto } from "../wailsjs/go/models";
	import EntryIcon from "./lib/EntryIcon.svelte";
	import PathBreadcrumb from "./lib/PathBreadcrumb.svelte";
	import SearchField from "./lib/SearchField.svelte";
	import SearchResults from "./lib/SearchResults.svelte";
	import { formatFileSize, formatModifiedTime, parentDirectoryOf } from "./lib/format";

	// Long enough that typing a word does not fire a query per keystroke, short
	// enough that results feel immediate.
	const searchDebounceMilliseconds = 150;
	const statusPollMilliseconds = 3000;

	type SearchOutcome = {
		hits: proto.Hit[];
		truncated: boolean;
		errorMessage: string;
	};

	let listing = $state<filesystem.Listing | null>(null);
	let errorMessage = $state("");
	let isLoading = $state(false);
	let showHidden = $state(false);

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
	const isSearchActive = $derived(searchQuery.trim().length > 0);
	const isIndexing = $derived(indexStatus?.state === "scanning");

	onMount(() => {
		navigateTo("");
	});

	$effect(() => {
		const query = searchQuery.trim();
		if (!query) {
			clearSearch();
			return;
		}
		const timer = setTimeout(() => runSearch(query), searchDebounceMilliseconds);
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

	async function navigateTo(path: string) {
		isLoading = true;
		errorMessage = "";
		try {
			listing = await ListDirectory(path);
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
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

	function openEntry(entry: filesystem.Entry) {
		if (!entry.isDir) {
			return;
		}
		navigateTo(entry.path);
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

	function toggleHidden() {
		showHidden = !showHidden;
	}

	function hiddenToggleIcon(): IconComponent {
		if (showHidden) {
			return EyeIcon;
		}
		return EyeSlashIcon;
	}

	function entrySubtitle(entry: filesystem.Entry): string {
		const modified = formatModifiedTime(entry.modTime);
		if (entry.isDir) {
			return modified;
		}
		return `${formatFileSize(entry.size)} · ${modified}`;
	}

	function pathOf(current: filesystem.Listing | null): string {
		if (!current) {
			return "";
		}
		return current.path;
	}

	function describeError(error: unknown): string {
		if (error instanceof Error) {
			return error.message;
		}
		return String(error);
	}

	function itemCountLabel(count: number): string {
		if (count === 1) {
			return "1 item";
		}
		return `${count} items`;
	}

	function indexingLabel(status: proto.StatusResult): string {
		return `Indexing ${status.filesIndexed.toLocaleString()} entries so far`;
	}
</script>

<div class="flex h-full flex-col bg-canvas">
	<header class="flex items-center gap-2 border-b border-line px-4 py-3">
		<Button size="sm" variant="ghost" icon={ArrowUpIcon} disabled={!canGoUp} onclick={goUp} />
		<Button size="sm" variant="ghost" icon={HouseIcon} onclick={goHome} />

		<PathBreadcrumb path={currentPath} onNavigate={navigateTo} />

		<SearchField bind:value={searchQuery} />

		<Button size="sm" variant="ghost" icon={hiddenToggleIcon()} onclick={toggleHidden} />
	</header>

	<main class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
		{#if isSearchActive}
			{#if searchErrorMessage}
				<Card>
					<div class="flex items-center gap-3">
						<WarningIcon size={20} class="shrink-0 text-red" />
						<div class="min-w-0">
							<p class="text-sm font-medium text-default">Search is unavailable</p>
							<p class="truncate text-xs text-dim">{searchErrorMessage}</p>
						</div>
					</div>
				</Card>
			{:else if isSearching && searchHits.length === 0}
				<div class="flex justify-center py-12">
					<LoadingSpinner />
				</div>
			{:else}
				{#if isIndexing && indexStatus}
					<p class="mb-2 text-xs text-dim">{indexingLabel(indexStatus)}</p>
				{/if}
				{#if searchHits.length === 0}
					<SectionHeader title="No matches" />
				{:else}
					<SearchResults hits={searchHits} truncated={searchTruncated} onOpen={openHit} />
				{/if}
			{/if}
		{:else if errorMessage}
			<Card>
				<div class="flex items-center gap-3">
					<WarningIcon size={20} class="shrink-0 text-red" />
					<div class="min-w-0">
						<p class="text-sm font-medium text-default">Could not open this folder</p>
						<p class="truncate text-xs text-dim">{errorMessage}</p>
					</div>
				</div>
			</Card>
		{:else if isLoading}
			<div class="flex justify-center py-12">
				<LoadingSpinner />
			</div>
		{:else}
			<SectionHeader title={itemCountLabel(visibleEntries.length)} />
			<div class="mt-2 flex flex-col gap-1">
				{#each visibleEntries as entry (entry.path)}
					<ListRow
						title={entry.name}
						subtitle={entrySubtitle(entry)}
						chevron={entry.isDir}
						onclick={() => openEntry(entry)}
					>
						{#snippet leading()}
							<EntryIcon {entry} />
						{/snippet}
					</ListRow>
				{/each}
			</div>
		{/if}
	</main>
</div>
