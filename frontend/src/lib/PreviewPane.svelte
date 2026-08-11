<script lang="ts">
	import { Card, LoadingSpinner } from "@neoworks-dev/ui";
	import {
		CancelMeasurement,
		MeasureFolder,
		PreviewFile,
	} from "../../wailsjs/go/main/FileService";
	import type { filesystem } from "../../wailsjs/go/models";
	import { EventsOn } from "../../wailsjs/runtime/runtime";
	import AudioPreview from "./AudioPreview.svelte";
	import CodeCanvas from "./CodeCanvas.svelte";
	import EntryIcon from "./EntryIcon.svelte";
	import { describeError } from "./errors";
	import { classifyEntry, kindLabel } from "./fileKind";
	import {
		measuredContentsLabel,
		measuredSizeLabel,
		usageEvents,
		type Measurement,
	} from "./folderSize";
	import { formatFileSize, formatModifiedTime } from "./format";

	let { entries, width }: { entries: filesystem.Entry[]; width: number } = $props();

	let preview = $state<filesystem.Preview | null>(null);
	let errorMessage = $state("");
	let isLoading = $state(false);
	let measurement = $state<Measurement | null>(null);
	let isCounting = $state(false);

	// Guards against a slow preview of an earlier selection landing after a
	// faster one that replaced it.
	let latestRequest = 0;

	// The measurement the size row is following, and the counter that tells a
	// walk started for a selection nobody is looking at any more.
	let measurementID = "";
	let latestMeasurement = 0;

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

	$effect(() => {
		const entry = singleEntry;
		stopMeasuring();
		if (!entry || !entry.isDir) {
			return;
		}
		startMeasuring(entry.path);
	});

	$effect(() => {
		const stops = [
			EventsOn(usageEvents.progress, (update: Measurement) => applyMeasurement(update, true)),
			EventsOn(usageEvents.finished, (update: Measurement) => applyMeasurement(update, false)),
		];
		return () => stops.forEach((stop) => stop());
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

	async function startMeasuring(path: string) {
		const request = ++latestMeasurement;
		measurement = null;
		isCounting = true;
		const started = await MeasureFolder(path).catch(() => "");
		adoptMeasurement(request, started);
	}

	// A walk whose selection is already gone by the time it reports its id has
	// nobody to count for, so it is called off rather than followed.
	function adoptMeasurement(request: number, started: string) {
		if (request !== latestMeasurement) {
			cancelMeasurement(started);
			return;
		}
		measurementID = started;
		isCounting = started !== "";
	}

	function stopMeasuring() {
		latestMeasurement++;
		cancelMeasurement(measurementID);
		measurementID = "";
		measurement = null;
		isCounting = false;
	}

	// A measurement that has already finished is unknown to the scanner, which is
	// nothing the pane has to react to.
	function cancelMeasurement(id: string) {
		if (!id) {
			return;
		}
		CancelMeasurement(id).catch(() => {});
	}

	function applyMeasurement(update: Measurement, counting: boolean) {
		if (update.id !== measurementID || update.cancelled) {
			return;
		}
		measurement = update;
		isCounting = counting;
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
			...contentsRows(entry),
			{ label: "Modified", value: formatModifiedTime(entry.modTime) },
			{ label: "Permissions", value: entry.mode },
		];
	}

	function contentsRows(entry: filesystem.Entry): Array<{ label: string; value: string }> {
		if (!entry.isDir) {
			return [];
		}
		return [{ label: "Contents", value: measuredContentsLabel(measurement) }];
	}

	// A folder's own size says nothing about what it holds, so the row follows the
	// walk instead.
	function sizeLabel(entry: filesystem.Entry): string {
		if (entry.isDir) {
			return measuredSizeLabel(measurement, isCounting);
		}
		return formatFileSize(entry.size);
	}
</script>

<div class="flex shrink-0 flex-col" style="width: {width}px">
	<Card
		surface="elevated"
		padding="none"
		class="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden p-2 shadow-lg"
	>
		{#if singleEntry}
			<div class="flex shrink-0 items-center gap-2">
				<EntryIcon entry={singleEntry} size="md" />
				<div class="min-w-0">
					<p class="truncate text-sm font-medium text-default">{singleEntry.name}</p>
					<p class="truncate text-2xs text-faint">{singleEntry.path}</p>
				</div>
			</div>

			<dl class="flex shrink-0 flex-col gap-2 rounded-lg bg-raised p-2">
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
					class="min-h-0 flex-1 rounded-lg bg-raised object-contain p-2"
				/>
			{:else if preview?.kind === "audio"}
				<AudioPreview
					path={singleEntry.path}
					name={singleEntry.name}
					details={preview.audio}
					coverUrl={preview.dataUrl}
				/>
			{:else if preview?.kind === "text" && preview.tokens?.length}
				<CodeCanvas tokens={preview.tokens} />
				{#if preview.truncated}
					<p class="shrink-0 text-2xs text-faint">Preview shows the start of the file only.</p>
				{/if}
			{:else if preview?.kind === "text"}
				<pre
					class="min-h-0 flex-1 overflow-auto rounded-lg bg-raised p-2 font-mono text-2xs whitespace-pre-wrap text-muted">{preview.text}</pre>
				{#if preview.truncated}
					<p class="shrink-0 text-2xs text-faint">Preview shows the start of the file only.</p>
				{/if}
			{:else if preview?.kind === "unsupported"}
				<p class="text-xs text-dim">No preview for this file type.</p>
			{/if}
		{:else if entries.length === 0}
			<p class="m-auto px-2 text-center text-xs text-faint">
				Select an item to see its details
			</p>
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
</div>
