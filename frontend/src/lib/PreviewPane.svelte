<script lang="ts">
	import { Card, LoadingSpinner } from "@neoworks-dev/ui";
	import { PreviewFile } from "../../wailsjs/go/main/FileService";
	import type { filesystem } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { describeError } from "./errors";
	import { classifyEntry, kindLabel } from "./fileKind";
	import { formatFileSize, formatModifiedTime } from "./format";

	let { entries }: { entries: filesystem.Entry[] } = $props();

	let preview = $state<filesystem.Preview | null>(null);
	let errorMessage = $state("");
	let isLoading = $state(false);

	// Guards against a slow preview of an earlier selection landing after a
	// faster one that replaced it.
	let latestRequest = 0;

	const singleEntry = $derived(onlyEntry(entries));
	const totalSize = $derived(entries.reduce((sum, entry) => sum + entry.size, 0));

	$effect(() => {
		const entry = singleEntry;
		if (!entry) {
			resetPreview();
			return;
		}
		loadPreview(entry.path);
	});

	async function loadPreview(path: string) {
		const request = ++latestRequest;
		isLoading = true;
		errorMessage = "";
		try {
			applyPreview(request, await PreviewFile(path), "");
		} catch (error) {
			applyPreview(request, null, describeError(error));
		}
	}

	function applyPreview(
		request: number,
		loaded: filesystem.Preview | null,
		message: string,
	) {
		if (request !== latestRequest) {
			return;
		}
		preview = loaded;
		errorMessage = message;
		isLoading = false;
	}

	function resetPreview() {
		latestRequest++;
		preview = null;
		errorMessage = "";
		isLoading = false;
	}

	function onlyEntry(selected: filesystem.Entry[]): filesystem.Entry | null {
		if (selected.length !== 1) {
			return null;
		}
		return selected[0];
	}

	function metadataRows(entry: filesystem.Entry): Array<{ label: string; value: string }> {
		return [
			{ label: "Type", value: kindLabel(classifyEntry(entry)) },
			{ label: "Size", value: sizeLabel(entry) },
			{ label: "Modified", value: formatModifiedTime(entry.modTime) },
			{ label: "Permissions", value: entry.mode },
		];
	}

	function sizeLabel(entry: filesystem.Entry): string {
		if (entry.isDir) {
			return "—";
		}
		return formatFileSize(entry.size);
	}
</script>

<Card
	surface="elevated"
	padding="none"
	class="flex w-80 shrink-0 flex-col gap-2 overflow-y-auto p-2 shadow-lg"
>
	{#if singleEntry}
		<div class="flex items-center gap-2">
			<EntryIcon entry={singleEntry} size="md" />
			<div class="min-w-0">
				<p class="truncate text-sm font-medium text-default">{singleEntry.name}</p>
				<p class="truncate text-2xs text-faint">{singleEntry.path}</p>
			</div>
		</div>

		<dl class="flex flex-col gap-2 rounded-lg bg-raised p-2">
			{#each metadataRows(singleEntry) as row (row.label)}
				<div class="flex items-baseline justify-between gap-2">
					<dt class="text-2xs tracking-caps text-faint uppercase">{row.label}</dt>
					<dd class="truncate text-xs text-muted">{row.value}</dd>
				</div>
			{/each}
		</dl>

		{#if errorMessage}
			<p class="text-xs text-red">{errorMessage}</p>
		{:else if isLoading}
			<div class="flex justify-center p-2">
				<LoadingSpinner />
			</div>
		{:else if preview?.kind === "image"}
			<img
				src={preview.dataUrl}
				alt={singleEntry.name}
				class="max-h-80 w-full rounded-lg bg-raised object-contain p-2"
			/>
		{:else if preview?.kind === "text"}
			<pre
				class="max-h-96 overflow-auto rounded-lg bg-raised p-2 font-mono text-2xs whitespace-pre-wrap text-muted">{preview.text}</pre>
			{#if preview.truncated}
				<p class="text-2xs text-faint">Preview shows the start of the file only.</p>
			{/if}
		{:else if preview?.kind === "unsupported"}
			<p class="text-xs text-dim">No preview for this file type.</p>
		{/if}
	{:else}
		<p class="text-sm font-medium text-default">{entries.length} items selected</p>
		<p class="text-xs text-dim">{formatFileSize(totalSize)} in total</p>
		<div class="flex min-h-0 flex-col gap-2 overflow-y-auto">
			{#each entries as entry (entry.path)}
				<div class="flex items-center gap-2">
					<EntryIcon {entry} />
					<span class="min-w-0 flex-1 truncate text-xs text-muted">{entry.name}</span>
				</div>
			{/each}
		</div>
	{/if}
</Card>
