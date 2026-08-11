<script lang="ts">
	import { Button, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import ArrowSquareOutIcon from "phosphor-svelte/lib/ArrowSquareOutIcon";
	import { FilesWithTag } from "../../wailsjs/go/main/FileService";
	import type { filesystem } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { describeError } from "./errors";
	import { formatFileSize, formatModifiedTime, parentDirectoryOf } from "./format";

	let {
		name,
		onOpen,
		onReveal,
	}: {
		name: string;
		onOpen: (entry: filesystem.Entry) => void;
		onReveal: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state<filesystem.Entry[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);

	// Guards against a slow walk for one tag landing after the user has already
	// picked another.
	let latestSearch = 0;

	$effect(() => {
		search(name);
	});

	async function search(tag: string) {
		const sequence = ++latestSearch;
		isLoading = true;
		errorMessage = "";
		try {
			applyResults(sequence, await FilesWithTag(tag), "");
		} catch (error) {
			applyResults(sequence, [], describeError(error));
		}
	}

	function applyResults(sequence: number, found: filesystem.Entry[], message: string) {
		if (sequence !== latestSearch) {
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
	<SectionHeader title={name} />
	<div class="flex-1"></div>
	<span class="pb-2 text-xs text-faint">{entries.length} tagged</span>
</div>

{#if errorMessage}
	<p class="text-xs text-red">{errorMessage}</p>
{/if}

{#if isLoading}
	<div class="flex flex-col items-center gap-2 p-2">
		<LoadingSpinner />
		<p class="text-2xs text-faint">
			Tags live on the files themselves, so this looks through your home folder
		</p>
	</div>
{:else if entries.length === 0}
	<p class="text-sm text-dim">Nothing in your home folder carries this tag.</p>
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
				<Button
					size="sm"
					variant="ghost"
					icon={ArrowSquareOutIcon}
					onclick={() => onReveal(entry)}
				/>
			</div>
		{/each}
	</div>
{/if}
