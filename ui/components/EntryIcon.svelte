<!--
	What stands for an entry in a listing: its thumbnail where one can be made,
	otherwise an icon for its kind in a tinted circle. A tile is the same size
	either way, which is what the list's row height counts on.
-->
<script lang="ts">
	import { Image, View } from "@neoworks/svelte-gpui/ui";
	import IconCircle, { type Tone } from "./design/IconCircle.svelte";
	import { file, fileCode, filePdf, fileText, fileZip, folder, image, musicNote, videoCamera } from "../icons";
	import { classifyEntry, type FileKind } from "../lib/fileKind";
	import { cachedThumbnail, loadThumbnail, supportsThumbnail, type ThumbnailSource } from "../lib/thumbnails";
	import { radius } from "../theme";

	type IconSize = "sm" | "md" | "lg" | "xl";

	let {
		entry,
		size = "sm",
	}: {
		entry: ThumbnailSource;
		size?: IconSize;
	} = $props();

	const iconsByKind: Record<FileKind, string> = {
		folder,
		image,
		audio: musicNote,
		video: videoCamera,
		archive: fileZip,
		pdf: filePdf,
		code: fileCode,
		text: fileText,
		file,
	};

	const tonesByKind: Record<FileKind, Tone> = {
		folder: "blue",
		image: "violet",
		audio: "amber",
		video: "amber",
		archive: "neutral",
		pdf: "red",
		code: "green",
		text: "neutral",
		file: "neutral",
	};

	// IconCircle stops at 48px and holds an icon rather than an image, so the box a
	// preview fills is sized here and the circle is kept inside it.
	const boxes: Record<IconSize, { size: number; radius: number }> = {
		sm: { size: 32, radius: radius.md },
		md: { size: 40, radius: radius.md },
		lg: { size: 48, radius: radius.lg },
		xl: { size: 96, radius: radius.lg },
	};

	let loadedURL = $state("");

	// Guards against the preview of an entry this component has already been
	// handed a replacement for arriving late and overwriting it.
	let latestRequest = 0;

	const kind = $derived(classifyEntry(entry));
	const circleSize = $derived(circleSizeFor(size));
	// Falling back to what is already cached is what keeps a tile scrolled back
	// into view from flashing its icon before the image arrives again.
	const thumbnailURL = $derived(previewOf(entry, loadedURL));

	$effect(() => {
		const source = entry;
		const request = ++latestRequest;
		loadedURL = "";
		if (!supportsThumbnail(source)) {
			return;
		}
		loadThumbnail(source).then((dataURL) => applyThumbnail(request, dataURL));
	});

	function previewOf(source: ThumbnailSource, loaded: string): string {
		if (loaded) {
			return loaded;
		}
		return cachedThumbnail(source);
	}

	function applyThumbnail(request: number, dataURL: string) {
		if (request !== latestRequest) {
			return;
		}
		loadedURL = dataURL;
	}

	function circleSizeFor(requested: IconSize): "sm" | "md" | "lg" {
		if (requested === "xl") {
			return "lg";
		}
		return requested;
	}
</script>

{#if thumbnailURL}
	<Image src={thumbnailURL} fit="contain" style={{ ...boxes[size], shrink: 0 }} />
{:else}
	<View style={{ ...boxes[size], shrink: 0, align: "center", justify: "center" }}>
		<IconCircle icon={iconsByKind[kind]} tone={tonesByKind[kind]} size={circleSize} />
	</View>
{/if}
