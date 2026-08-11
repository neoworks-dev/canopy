<script lang="ts">
	import { Button, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
	import { SearchEntries } from "../../wailsjs/go/main/IndexService";
	import type { filesystem } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { describeError } from "./errors";
	import { formatFileSize, formatModifiedTime, parentDirectoryOf } from "./format";
	import { searchDescription, type SavedSearch } from "./savedSearches";

	// A saved search shown as a folder: the query is run again when the pane
	// opens, so what it holds is whatever matches now rather than what it matched
	// when it was saved.
	let {
		search,
		onOpen,
		onReveal,
	}: {
		search: SavedSearch;
		onOpen: (entry: filesystem.Entry) => void;
		onReveal: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state<filesystem.Entry[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);

	// Guards against a slow query landing after the user has opened another.
	let latestRun = 0;

	$effect(() => {
		run(search);
	});

	async function run(wanted: SavedSearch) {
		const sequence = ++latestRun;
		isLoading = true;
		errorMessage = "";
		try {
			applyResults(sequence, await SearchEntries(wanted.query, wanted.scope), "");
		} catch (error) {
			applyResults(sequence, [], describeError(error));
		}
	}

	function applyResults(sequence: number, found: filesystem.Entry[], message: string) {
		if (sequence !== latestRun) {
			return;
		}
		entries = found;
		errorMessage = message;
		isLoading = false;
	}

	function sizeLabel(entry: filesystem.Entry): string {
		if (entry.isDir) {
			return "—";
		}
		return formatFileSize(entry.size);
	}
</script>

<div class="flex items-center gap-2">
	<SectionHeader title={search.name} />
	<div class="flex-1"></div>
	<span class="pb-2 text-xs text-faint">{entries.length} matching</span>
</div>

<p class="truncate text-2xs text-faint">{searchDescription(search)}</p>

{#if errorMessage}
	<p class="text-xs text-red">{errorMessage}</p>
{/if}

{#if isLoading}
	<div class="flex justify-center p-2">
		<LoadingSpinner />
	</div>
{:else if entries.length === 0}
	<p class="text-sm text-dim">Nothing matches this search right now.</p>
{:else}
	<div class="flex min-w-0 flex-col gap-2 overflow-y-auto">
		{#each entries as entry (entry.path)}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				ondblclick={() => onOpen(entry)}
				class="flex items-center gap-2 rounded-lg p-2 hover:bg-hover"
			>
				<EntryIcon {entry} />
				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-medium text-default">{entry.name}</p>
					<p class="truncate text-2xs text-faint">{parentDirectoryOf(entry.path)}</p>
				</div>
				<span class="shrink-0 text-xs text-dim tabular-nums">{sizeLabel(entry)}</span>
				<span class="shrink-0 text-xs text-dim">{formatModifiedTime(entry.modTime)}</span>
				<Button size="sm" variant="ghost" icon={ArrowSquareOutIcon} onclick={() => onReveal(entry)} />
			</div>
		{/each}
	</div>
{/if}
