<!--
	One entry of a listing as something that can be picked up and, when it is a
	folder, dropped on. The row, tile or column entry to draw goes inside, and is
	told whether a drag is over it. What a drag carries is `{ paths }`, which is
	what the sidebar's drop targets read too.

	svelte-gpui: dragging entries out to other programs is not supported (issue
	#23), so a drag only reaches targets inside this window.
-->
<script lang="ts">
	import type { Snippet } from "svelte";
	import { Draggable, DropTarget } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";

	type DraggedPaths = { paths: string[] };

	let {
		entry,
		dragPaths,
		isDragDisabled = false,
		onDragStart,
		onDragEnd,
		onDropOnFolder,
		canDropInto,
		children: content,
	}: {
		entry: filesystem.Entry;
		dragPaths: string[];
		/** An entry being renamed is not one to drag by its name field. */
		isDragDisabled?: boolean;
		onDragStart: () => void;
		onDragEnd: () => void;
		onDropOnFolder: (folder: filesystem.Entry, sources: string[]) => void;
		canDropInto: (folder: string, sources: string[]) => boolean;
		children: Snippet<[boolean]>;
	} = $props();

	// The folder being dragged, the folder the drag came from and anything inside
	// what is dragged would take nothing, so they do not light up.
	function acceptsPaths(data: DraggedPaths): boolean {
		if (!Array.isArray(data?.paths)) {
			return false;
		}
		return canDropInto(entry.path, data.paths);
	}

	function drop(data: DraggedPaths) {
		onDropOnFolder(entry, data.paths);
	}
</script>

{#snippet draggable(isDropOver: boolean)}
	<Draggable
		data={{ paths: dragPaths }}
		disabled={isDragDisabled}
		style={{ flex: 1, minWidth: 0 }}
		onstart={onDragStart}
		onend={onDragEnd}
	>
		{@render content(isDropOver)}
	</Draggable>
{/snippet}

{#if entry.isDir}
	<DropTarget style={{ flex: 1, minWidth: 0, direction: "row" }} accept={acceptsPaths} ondrop={({ data }) => drop(data)}>
		{#snippet children({ over })}
			{@render draggable(over)}
		{/snippet}
	</DropTarget>
{:else}
	{@render draggable(false)}
{/if}
