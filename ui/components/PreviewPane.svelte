<!--
	The details pane: what the selection is, and for one file a preview of its
	contents. A folder's size is counted in the background and follows the walk.
	With several entries selected it lists them and adds their sizes up.
-->
<script lang="ts">
	import { Image, List, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
	import { CancelMeasurement, MeasureFolder, PreviewFile } from "../backend/FileService";
	import { EventsOn } from "../backend/events";
	import type { filesystem } from "../backend/models";
	import { describeError } from "../lib/errors";
	import { classifyEntry, kindLabel } from "../lib/fileKind";
	import {
		measuredContentsLabel,
		measuredSizeLabel,
		usageEvents,
		type Measurement,
	} from "../lib/folderSize";
	import { formatFileSize, formatModifiedTime } from "../lib/format";
	import { colors, radius, space, textSize } from "../theme";
	import AudioPreview from "./AudioPreview.svelte";
	import CodeCanvas from "./CodeCanvas.svelte";
	import EntryIcon from "./EntryIcon.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";

	let { entries, width }: { entries: filesystem.Entry[]; width: number } = $props();

	let preview = $state<filesystem.Preview | null>(null);
	let errorMessage = $state("");
	let isLoading = $state(false);
	let measurement = $state<Measurement | null>(null);
	let isCounting = $state(false);
	let codeCanvas = $state<CodeCanvas>();

	/** Scrolls a text preview by a number of lines; other previews do not move. */
	export function seek(lineCount: number) {
		codeCanvas?.seek(lineCount);
	}

	// Guards against a slow preview of an earlier selection landing after a
	// faster one that replaced it.
	let latestRequest = 0;

	// The measurement the size row is following, and the counter that tells a
	// walk started for a selection nobody is looking at any more.
	let measurementID = "";
	let latestMeasurement = 0;

	const singleEntry = $derived(onlyEntry(entries));
	const totalSize = $derived(entries.reduce((sum, entry) => sum + entry.size, 0));
	const rows = $derived(metadataRows(singleEntry, measurement, isCounting));

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

	function applyPreview(request: number, loaded: filesystem.Preview | null, message: string) {
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

	type MetadataRow = { label: string; value: string };

	function metadataRows(entry: filesystem.Entry | null, measured: Measurement | null, counting: boolean): MetadataRow[] {
		if (!entry) {
			return [];
		}
		return [
			{ label: "Type", value: kindLabel(classifyEntry(entry)) },
			{ label: "Size", value: sizeLabel(entry, measured, counting) },
			...contentsRows(entry, measured),
			{ label: "Modified", value: formatModifiedTime(entry.modTime) },
			{ label: "Permissions", value: entry.mode },
		];
	}

	function contentsRows(entry: filesystem.Entry, measured: Measurement | null): MetadataRow[] {
		if (!entry.isDir) {
			return [];
		}
		return [{ label: "Contents", value: measuredContentsLabel(measured) }];
	}

	// A folder's own size says nothing about what it holds, so the row follows the
	// walk instead.
	function sizeLabel(entry: filesystem.Entry, measured: Measurement | null, counting: boolean): string {
		if (entry.isDir) {
			return measuredSizeLabel(measured, counting);
		}
		return formatFileSize(entry.size);
	}

	// Source that no grammar covers is plain text: one token, wrapped.
	function plainTokens(text: string): { class: string; text: string }[] {
		return [{ class: "", text }];
	}

	function hasTokens(loaded: filesystem.Preview): boolean {
		return Array.isArray(loaded.tokens) && loaded.tokens.length > 0;
	}
</script>

{#snippet truncationNote()}
	<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>
		Preview shows the start of the file only.
	</Text>
{/snippet}

<View
	style={{
		width,
		shrink: 0,
		gap: space[2],
		padding: space[2],
		overflow: "hidden",
		borderLeftWidth: 1,
		borderColor: colors.lineFaint,
	}}
>
	{#if singleEntry}
		<View style={{ direction: "row", shrink: 0, align: "center", gap: space[2] }}>
			<EntryIcon entry={singleEntry} size="md" />
			<View style={{ flex: 1, minWidth: 0 }}>
				<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
					{singleEntry.name}
				</Text>
				<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>{singleEntry.path}</Text>
			</View>
		</View>

		<View style={{ shrink: 0, gap: space[2], padding: space[2], radius: radius.lg, background: colors.raised }}>
			{#each rows as row (row.label)}
				<View style={{ direction: "row", align: "baseline", justify: "between", gap: space[2] }}>
					<Text style={{ fontSize: textSize["2xs"], uppercase: true, color: colors.faint }}>{row.label}</Text>
					<Text style={{ flex: 1, minWidth: 0, truncate: true, textAlign: "right", fontSize: textSize.xs, color: colors.muted }}>
						{row.value}
					</Text>
				</View>
			{/each}
		</View>

		{#if errorMessage}
			<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
		{:else if isLoading}
			<View style={{ align: "center", padding: space[2] }}><LoadingSpinner /></View>
		{:else if preview?.kind === "image"}
			<View style={{ flex: 1, minHeight: 0, padding: space[2], radius: radius.lg, background: colors.raised }}>
				<Image src={preview.dataUrl} fit="contain" style={{ width: "full", height: "full" }} />
			</View>
		{:else if preview?.kind === "audio"}
			<Scroll style={{ flex: 1, minHeight: 0 }}>
				<AudioPreview details={preview.audio} coverUrl={preview.dataUrl} />
			</Scroll>
		{:else if preview?.kind === "text" && hasTokens(preview)}
			{#key preview.path}
				<CodeCanvas bind:this={codeCanvas} tokens={preview.tokens} />
			{/key}
			{#if preview.truncated}
				{@render truncationNote()}
			{/if}
		{:else if preview?.kind === "text"}
			{#key preview.path}
				<CodeCanvas bind:this={codeCanvas} tokens={plainTokens(preview.text)} isWrapped />
			{/key}
			{#if preview.truncated}
				{@render truncationNote()}
			{/if}
		{:else if preview?.kind === "unsupported"}
			<Text style={{ fontSize: textSize.xs, color: colors.dim }}>No preview for this file type.</Text>
		{/if}
	{:else if entries.length === 0}
		<View style={{ flex: 1, align: "center", justify: "center", paddingX: space[2] }}>
			<Text style={{ textAlign: "center", fontSize: textSize.xs, color: colors.faint }}>
				Select an item to see its details
			</Text>
		</View>
	{:else}
		<Text style={{ shrink: 0, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
			{entries.length} items selected
		</Text>
		<Text style={{ shrink: 0, fontSize: textSize.xs, color: colors.dim }}>
			{formatFileSize(totalSize)} in total
		</Text>
		<List items={entries} rowHeight={40} key={(entry) => entry.path} style={{ flex: 1, minHeight: 0 }}>
			{#snippet row(entry)}
				<View style={{ direction: "row", align: "center", gap: space[2], height: 40 }}>
					<EntryIcon {entry} />
					<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.xs, color: colors.muted }}>
						{entry.name}
					</Text>
				</View>
			{/snippet}
		</List>
	{/if}
</View>
