<!--
	One entry in a pane of found files: its icon, name and folder, then its size
	and time, and a button that shows it where it lives. Pressing the row twice
	opens it.
-->
<script lang="ts">
	import { Text, View } from "@neoworks/svelte-gpui/ui";
	import { arrowSquareOut } from "../icons";
	import type { filesystem } from "../backend/models";
	import { formatFileSize, formatModifiedTime, parentDirectoryOf } from "../lib/format";
	import { colors, duration, radius, space, textSize } from "../theme";
	import EntryIcon from "./EntryIcon.svelte";
	import IconButton from "./IconButton.svelte";

	let {
		entry,
		onOpen,
		onReveal,
	}: {
		entry: filesystem.Entry;
		onOpen: (entry: filesystem.Entry) => void;
		onReveal: (entry: filesystem.Entry) => void;
	} = $props();

	function openOnDoublePress(clicks: number) {
		if (clicks < 2) {
			return;
		}
		onOpen(entry);
	}

	function sizeLabel(): string {
		if (entry.isDir) {
			return "—";
		}
		return formatFileSize(entry.size);
	}
</script>

<View
	style={{
		direction: "row",
		align: "center",
		gap: space[2],
		padding: space[2],
		radius: radius.lg,
		transition: duration.fast,
	}}
	hover={{ background: colors.hover }}
	onpress={(event) => openOnDoublePress(event.clicks)}
>
	<EntryIcon {entry} />
	<View style={{ flex: 1, minWidth: 0 }}>
		<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
			{entry.name}
		</Text>
		<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>
			{parentDirectoryOf(entry.path)}
		</Text>
	</View>
	<Text style={{ shrink: 0, fontSize: textSize.xs, color: colors.dim }}>{sizeLabel()}</Text>
	<Text style={{ shrink: 0, fontSize: textSize.xs, color: colors.dim }}>{formatModifiedTime(entry.modTime)}</Text>
	<IconButton icon={arrowSquareOut} size={16} title="Show in folder" onpress={() => onReveal(entry)} />
</View>
