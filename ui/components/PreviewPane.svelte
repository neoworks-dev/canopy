<!--
	The details pane: what the selection is, and for one file a preview of its
	contents. A folder's size is counted in the background and follows the walk.
	With several entries selected it lists them and adds their sizes up.
-->
<script lang="ts">
	import { Image, List, Scroll, Text, View, type Style } from "@neoworks/svelte-gpui/ui";
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
	import { entryRowHeight, kindColor } from "../lib/entryStyles";
	import type { SortOrder } from "../lib/sorting";
	import { colors, radius, space, textSize } from "../theme";
	import AudioPreview from "./AudioPreview.svelte";
	import CodeCanvas from "./CodeCanvas.svelte";
	import EntryIcon from "./EntryIcon.svelte";
	import MillerColumn from "./MillerColumn.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";

	let {
		entries,
		width,
		share = 0,
		sortOrder,
		showHidden,
		onOpenFolder,
	}: {
		entries: filesystem.Entry[];
		/** The width it was dragged to, used while it has no share. */
		width: number;
		/** Its part of the row's width against the listing's, as Yazi's 3 of 1:4:3; 0 keeps `width`. */
		share?: number;
		sortOrder: SortOrder;
		showHidden: boolean;
		onOpenFolder: (path: string) => void;
	} = $props();

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
	const footer = $derived(footerLine(singleEntry, measurement, isCounting));

	// A folder previews as its listing, which the column reads for itself.
	$effect(() => {
		const entry = singleEntry;
		if (!entry || entry.isDir) {
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

	/** One line under the preview: what it is, how big, and when it last changed. */
	function footerLine(entry: filesystem.Entry | null, measured: Measurement | null, counting: boolean): string {
		if (!entry) {
			return "";
		}
		return [
			kindLabel(classifyEntry(entry)),
			sizeLabel(entry, measured, counting),
			...contentsParts(entry, measured),
			formatModifiedTime(entry.modTime),
		].join(" · ");
	}

	function contentsParts(entry: filesystem.Entry, measured: Measurement | null): string[] {
		if (!entry.isDir) {
			return [];
		}
		return [measuredContentsLabel(measured)];
	}

	function paneSize(): Style {
		if (share > 0) {
			return { flex: share, minWidth: 0 };
		}
		return { width, shrink: 0 };
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

<View style={{ ...paneSize(), overflow: "hidden" }}>
	{#if singleEntry?.isDir}
		{#key singleEntry.path}
			<MillerColumn
				path={singleEntry.path}
				{sortOrder}
				{showHidden}
				onOpen={(entry) => onOpenFolder(entry.path)}
			/>
		{/key}
	{:else if singleEntry}
		<View style={{ flex: 1, minHeight: 0, gap: space[2], padding: space[2] }}>
		{#if errorMessage}
			<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
		{:else if isLoading}
			<View style={{ align: "center", padding: space[2] }}><LoadingSpinner /></View>
		{:else if preview?.kind === "image"}
			<View style={{ flex: 1, minHeight: 0 }}>
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
		</View>
	{:else if entries.length === 0}
		<View style={{ flex: 1, align: "center", justify: "center", paddingX: space[2] }}>
			<Text style={{ textAlign: "center", fontSize: textSize.xs, color: colors.faint }}>
				Select an item to see its details
			</Text>
		</View>
	{:else}
		<View style={{ flex: 1, minHeight: 0, gap: space[1], padding: space[2] }}>
			<Text style={{ shrink: 0, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
				{entries.length} items selected
			</Text>
			<Text style={{ shrink: 0, fontSize: textSize.xs, color: colors.dim }}>
				{formatFileSize(totalSize)} in total
			</Text>
			<List items={entries} rowHeight={entryRowHeight} key={(entry) => entry.path} style={{ flex: 1, minHeight: 0 }}>
				{#snippet row(entry)}
					<View style={{ direction: "row", align: "center", gap: space[2], height: entryRowHeight }}>
						<EntryIcon {entry} size="glyph" color={kindColor(classifyEntry(entry))} />
						<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.xs, color: colors.muted }}>
							{entry.name}
						</Text>
					</View>
				{/snippet}
			</List>
		</View>
	{/if}

	{#if footer}
		<Text
			style={{
				shrink: 0,
				truncate: true,
				paddingX: space[2],
				paddingY: space[1],
				borderTopWidth: 1,
				borderColor: colors.line,
				fontSize: textSize["2xs"],
				color: colors.dim,
			}}
		>
			{footer}
		</Text>
	{/if}
</View>
