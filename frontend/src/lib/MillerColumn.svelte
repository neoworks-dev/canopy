<script lang="ts">
	import { LoadingSpinner } from "@neoworks-dev/ui";
	import { ListDirectory } from "../../wailsjs/go/main/FileService";
	import type { filesystem } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { leadingChild } from "./millerColumns";
	import type { SortOrder } from "./sorting";

	// One column of a Miller chain: the entries of one folder, with the child that
	// leads on towards the folder on screen marked.
	let {
		path,
		currentPath,
		sortOrder,
		showHidden,
		onOpen,
	}: {
		path: string;
		/** The folder the pane is showing, which this column may lead towards. */
		currentPath: string;
		sortOrder: SortOrder;
		showHidden: boolean;
		onOpen: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state<filesystem.Entry[]>([]);
	let isLoading = $state(true);
	let hasFailed = $state(false);

	// Guards against a slow read of one folder landing after the chain moved on.
	let latestRead = 0;

	const openChild = $derived(leadingChild(path, currentPath));
	const visibleEntries = $derived(withoutHidden(entries));

	$effect(() => {
		read(path, sortOrder);
	});

	async function read(wanted: string, order: SortOrder) {
		const sequence = ++latestRead;
		isLoading = true;
		try {
			applyListing(sequence, await ListDirectory(wanted, order));
		} catch {
			applyFailure(sequence);
		}
	}

	function applyListing(sequence: number, listing: filesystem.Listing) {
		if (sequence !== latestRead) {
			return;
		}
		entries = listing.entries;
		hasFailed = false;
		isLoading = false;
	}

	function applyFailure(sequence: number) {
		if (sequence !== latestRead) {
			return;
		}
		entries = [];
		hasFailed = true;
		isLoading = false;
	}

	function withoutHidden(listed: filesystem.Entry[]): filesystem.Entry[] {
		if (showHidden) {
			return listed;
		}
		return listed.filter((entry) => !entry.name.startsWith("."));
	}
</script>

<div class="flex h-full w-56 shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-line-faint pr-2">
	{#if isLoading}
		<div class="flex justify-center p-2">
			<LoadingSpinner size={14} />
		</div>
	{:else if hasFailed}
		<p class="p-2 text-2xs text-faint">This folder could not be read.</p>
	{:else}
		{#each visibleEntries as entry (entry.path)}
			{@const isOpen = entry.path === openChild}
			<button
				type="button"
				onclick={() => onOpen(entry)}
				class="flex w-full shrink-0 items-center gap-2 rounded-md px-2 py-1 text-left transition duration-fast"
				class:bg-raised={isOpen}
				class:hover:bg-hover={!isOpen}
			>
				<EntryIcon {entry} />
				<span
					class="min-w-0 flex-1 truncate text-sm"
					class:text-default={isOpen}
					class:text-muted={!isOpen}
				>
					{entry.name}
				</span>
				{#if entry.isDir}
					<span class="shrink-0 text-2xs text-faint">›</span>
				{/if}
			</button>
		{/each}
		{#if visibleEntries.length === 0}
			<p class="p-2 text-2xs text-faint">Empty</p>
		{/if}
	{/if}
</div>
