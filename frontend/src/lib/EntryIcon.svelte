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

	let {
		entry,
		size = "sm",
	}: {
		entry: { name: string; isDir: boolean };
		size?: "sm" | "md" | "lg";
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

	const kind = $derived(classifyEntry(entry));
</script>

<IconCircle icon={iconsByKind[kind]} tone={tonesByKind[kind]} {size} />
