<script lang="ts">
	import { IconCircle } from "@neoworks-dev/ui";
	import type { IconComponent, StatusTone } from "@neoworks-dev/ui";
	import FolderIcon from "phosphor-svelte/lib/FolderIcon";
	import FileIcon from "phosphor-svelte/lib/FileIcon";
	import FileTextIcon from "phosphor-svelte/lib/FileTextIcon";
	import FileCodeIcon from "phosphor-svelte/lib/FileCodeIcon";
	import FileZipIcon from "phosphor-svelte/lib/FileZipIcon";
	import FilePdfIcon from "phosphor-svelte/lib/FilePdfIcon";
	import ImageIcon from "phosphor-svelte/lib/ImageIcon";
	import MusicNoteIcon from "phosphor-svelte/lib/MusicNoteIcon";
	import VideoCameraIcon from "phosphor-svelte/lib/VideoCameraIcon";
	import { classifyEntry, type FileKind } from "./fileKind";
	import {
		cachedThumbnail,
		loadThumbnail,
		supportsThumbnail,
		type ThumbnailSource,
	} from "./thumbnails";

	type IconSize = "sm" | "md" | "lg" | "xl";

	let {
		entry,
		size = "sm",
	}: {
		entry: ThumbnailSource;
		size?: IconSize;
	} = $props();

	const iconsByKind: Record<FileKind, IconComponent> = {
		folder: FolderIcon,
		image: ImageIcon,
		audio: MusicNoteIcon,
		video: VideoCameraIcon,
		archive: FileZipIcon,
		pdf: FilePdfIcon,
		code: FileCodeIcon,
		text: FileTextIcon,
		file: FileIcon,
	};

	const tonesByKind: Record<FileKind, StatusTone> = {
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

	// IconCircle from @neoworks-dev/ui stops at 48px and holds an icon rather than
	// an image, so the box a preview fills is sized here and the circle is kept
	// inside it. A tile is the same height either way, which is what the virtual
	// list measures.
	const boxClasses: Record<IconSize, string> = {
		sm: "h-8 w-8 rounded-lg",
		md: "h-10 w-10 rounded-lg",
		lg: "h-12 w-12 rounded-xl",
		xl: "h-24 w-24 rounded-xl",
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
	<img src={thumbnailURL} alt="" class="shrink-0 object-contain {boxClasses[size]}" />
{:else}
	<div class="flex shrink-0 items-center justify-center {boxClasses[size]}">
		<IconCircle icon={iconsByKind[kind]} tone={tonesByKind[kind]} size={circleSize} />
	</div>
{/if}
