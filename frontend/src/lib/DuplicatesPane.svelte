<script lang="ts">
	import { Button, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
	import TrashIcon from "phosphor-svelte/lib/TrashIcon";
	import XCircleIcon from "phosphor-svelte/lib/XCircleIcon";
	import {
		CancelDuplicateSearch,
		FindDuplicates,
		StartOperation,
	} from "../../wailsjs/go/main/FileService";
	import { EventsOn } from "../../wailsjs/runtime/runtime";
	import {
		duplicateEvents,
		extraPathsOf,
		groupLabel,
		recoverableLabel,
		remainingGroups,
		searchProgressLabel,
		type DuplicateGroup,
		type DuplicateReport,
	} from "./duplicates";
	import { describeError } from "./errors";
	import { parentDirectoryOf } from "./format";

	let {
		path,
		onOpen,
		onReveal,
		onChanged,
	}: {
		path: string;
		onOpen: (path: string) => void;
		/** Shows a copy where it lives, which moves this pane to its folder. */
		onReveal: (path: string) => void;
		/** Files were trashed from here, which the undo label follows. */
		onChanged: () => void;
	} = $props();

	let report = $state<DuplicateReport | null>(null);
	let errorMessage = $state("");
	let isSearching = $state(false);
	let trashed = $state(new Set<string>());

	// The search the pane is following, and the counter that tells a search
	// started for a folder it has already left.
	let searchID = "";
	let latestSearch = 0;

	// A small folder can be searched through before the call that started it has
	// even reported its id, so a report that arrives that early waits here until
	// there is an id to match it against.
	let earlyReport: { update: DuplicateReport; running: boolean } | null = null;

	const groups = $derived(remainingGroups(foundGroups(), trashed));

	$effect(() => {
		startSearch(path);
		return stopSearch;
	});

	$effect(() => {
		const stops = [
			EventsOn(duplicateEvents.progress, (update: DuplicateReport) => apply(update, true)),
			EventsOn(duplicateEvents.finished, (update: DuplicateReport) => apply(update, false)),
		];
		return () => stops.forEach((stop) => stop());
	});

	async function startSearch(root: string) {
		const request = ++latestSearch;
		report = null;
		earlyReport = null;
		trashed = new Set();
		errorMessage = "";
		isSearching = true;
		try {
			adopt(request, await FindDuplicates(root));
		} catch (error) {
			errorMessage = describeError(error);
			isSearching = false;
		}
	}

	// A search whose folder is already gone by the time it reports its id has
	// nobody to search for, so it is called off rather than followed.
	function adopt(request: number, started: string) {
		if (request !== latestSearch) {
			cancelSearch(started);
			return;
		}
		searchID = started;
		applyEarlyReport(started);
	}

	function applyEarlyReport(started: string) {
		const waiting = earlyReport;
		earlyReport = null;
		if (!waiting || waiting.update.id !== started) {
			return;
		}
		apply(waiting.update, waiting.running);
	}

	function stopSearch() {
		latestSearch++;
		cancelSearch(searchID);
		searchID = "";
		isSearching = false;
	}

	// A search that has already finished is unknown to the scanner, which is
	// nothing the pane has to react to.
	function cancelSearch(id: string) {
		if (!id) {
			return;
		}
		CancelDuplicateSearch(id).catch(() => {});
	}

	function apply(update: DuplicateReport, running: boolean) {
		if (!searchID) {
			earlyReport = { update, running };
			return;
		}
		if (update.id !== searchID) {
			return;
		}
		report = update;
		isSearching = running;
	}

	function foundGroups(): DuplicateGroup[] {
		if (!report) {
			return [];
		}
		return report.groups;
	}

	async function trash(paths: string[]) {
		if (paths.length === 0) {
			return;
		}
		try {
			await StartOperation("trash", paths, "");
		} catch (error) {
			errorMessage = describeError(error);
			return;
		}
		trashed = new Set([...trashed, ...paths]);
		onChanged();
	}

	function nameOf(filePath: string): string {
		const name = filePath.split("/").pop();
		if (!name) {
			return filePath;
		}
		return name;
	}

	function keyOf(group: DuplicateGroup): string {
		return group.paths[0];
	}
</script>

<div class="flex items-center gap-2">
	<SectionHeader title="Duplicates" />
	<span class="min-w-0 flex-1 truncate text-xs text-faint">{path}</span>
	{#if isSearching}
		<Button size="sm" variant="ghost" icon={XCircleIcon} onclick={stopSearch}>Stop</Button>
	{/if}
</div>

<p class="text-xs text-dim">
	{searchProgressLabel(report)}
	{#if report && report.stage === "done" && report.recoverableBytes > 0}
		· {recoverableLabel(report)}
	{/if}
</p>

{#if errorMessage}
	<p class="text-xs text-red">{errorMessage}</p>
{/if}

{#if isSearching && groups.length === 0}
	<div class="flex justify-center p-2"><LoadingSpinner /></div>
{/if}

<div class="flex min-w-0 flex-col gap-2">
	{#each groups as group (keyOf(group))}
		<div class="flex flex-col gap-1 rounded-lg bg-raised p-2">
			<div class="flex items-center gap-2">
				<p class="min-w-0 flex-1 truncate text-xs text-dim">{groupLabel(group)}</p>
				<Button size="sm" variant="ghost" icon={TrashIcon} onclick={() => trash(extraPathsOf(group))}>
					Keep the first
				</Button>
			</div>
			{#each group.paths as filePath (filePath)}
				<div class="flex items-center gap-2 rounded-lg p-1 hover:bg-hover">
					<button
						type="button"
						onclick={() => onOpen(filePath)}
						class="min-w-0 flex-1 text-left"
					>
						<p class="truncate text-sm font-medium text-default">{nameOf(filePath)}</p>
						<p class="truncate text-2xs text-faint">{parentDirectoryOf(filePath)}</p>
					</button>
					<Button
						size="sm"
						variant="ghost"
						icon={ArrowSquareOutIcon}
						onclick={() => onReveal(filePath)}
					/>
					<Button size="sm" variant="ghost" icon={TrashIcon} onclick={() => trash([filePath])} />
				</div>
			{/each}
		</div>
	{/each}
</div>
