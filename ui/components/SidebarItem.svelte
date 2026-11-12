<!--
	One row of the sidebar rail: an icon and a label that navigates, tools that
	appear while the row is hovered, and, when `onDropPaths` is given, a folder
	that takes drops. A drop is either a Draggable from this window's listing
	(`{ paths }` as its data) or files dragged in from another program. ListRow
	from the design system has no active state and its own padding scale, so
	this uses the same tokens at rail proportions.
-->
<script lang="ts">
	import {
		Button,
		DropTarget,
		Icon,
		Text,
		Tooltip,
		View,
		merge,
		type FileDragEvent,
	} from "@neoworks/svelte-gpui/ui";
	import { pencilSimple, x } from "../icons";
	import { dropOperation, type DropOperation } from "../lib/dragDrop";
	import { colors, duration, radius, space, textSize } from "../theme";
	import IconButton from "./IconButton.svelte";

	type DraggedPaths = { paths: string[] };

	let {
		icon,
		activeIcon = undefined,
		label,
		detail = "",
		title = "",
		active = false,
		onSelect,
		onRemove = undefined,
		removeLabel = "",
		onEdit = undefined,
		editLabel = "",
		onDropPaths = undefined,
	}: {
		/** SVG markup. */
		icon: string;
		/** Drawn instead of `icon` while the item is active. */
		activeIcon?: string;
		label: string;
		/** A short trailing value, such as the free space left on a volume. */
		detail?: string;
		title?: string;
		active?: boolean;
		onSelect: () => void;
		/** Present on the items that can be taken out of the sidebar. */
		onRemove?: () => void;
		removeLabel?: string;
		/** Present on the items that can be renamed in place. */
		onEdit?: () => void;
		editLabel?: string;
		/** Present on the items a drag can be dropped on. */
		onDropPaths?: (sources: string[], operation: DropOperation) => void;
	} = $props();

	// Drops carry no modifier keys, so a drag inside the window moves and one from
	// another program copies.
	const noModifiers = { ctrlKey: false, shiftKey: false, metaKey: false };

	let isHovered = $state(false);
	let isOutsideDragOver = $state(false);

	const shownIcon = $derived(iconFor(active));

	function iconFor(isActive: boolean): string {
		if (isActive && activeIcon) {
			return activeIcon;
		}
		return icon;
	}

	function acceptsPaths(data: DraggedPaths): boolean {
		return onDropPaths !== undefined && Array.isArray(data?.paths);
	}

	function dropFromWindow(data: DraggedPaths) {
		onDropPaths?.(data.paths, dropOperation("internal", noModifiers));
	}

	function enterOutsideDrag() {
		isOutsideDragOver = onDropPaths !== undefined;
	}

	function leaveOutsideDrag() {
		isOutsideDragOver = false;
	}

	function dropFromOutside(event: FileDragEvent) {
		isOutsideDragOver = false;
		onDropPaths?.(event.paths, dropOperation("external", noModifiers));
	}

	function rowLook(isDropOver: boolean) {
		return merge(
			{
				direction: "row",
				align: "center",
				radius: radius.md,
				borderWidth: 1,
				borderColor: "transparent",
			},
			active && { background: colors.raised },
			isDropOver && { borderColor: colors.action },
		);
	}

	// The active row already has its own background and full-strength text.
	function labelHover() {
		if (active) {
			return {};
		}
		return { background: colors.hover, color: colors.text };
	}

	function labelWeight() {
		if (active) {
			return "medium";
		}
		return "normal";
	}

	function labelLook() {
		return merge(
			{
				direction: "row",
				align: "center",
				gap: space[2],
				flex: 1,
				minWidth: 0,
				paddingX: space[2],
				paddingY: space[1.5],
				radius: radius.md,
				fontSize: textSize.sm,
				color: colors.muted,
				transition: duration.fast,
			},
			active && { color: colors.text },
		);
	}
</script>

{#snippet labelButton()}
	<Button
		style={labelLook()}
		hover={labelHover()}
		onpress={onSelect}
	>
		<Icon svg={shownIcon} size={16} />
		<Text style={{ flex: 1, minWidth: 0, truncate: true, fontWeight: labelWeight() }}>
			{label}
		</Text>
		{#if detail}
			<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>{detail}</Text>
		{/if}
	</Button>
{/snippet}

{#snippet row(isDropOver: boolean)}
	<View
		style={rowLook(isDropOver || isOutsideDragOver)}
		onhover={(hovered) => (isHovered = hovered)}
		ondragenter={enterOutsideDrag}
		ondragover={enterOutsideDrag}
		ondragleave={leaveOutsideDrag}
		ondrop={dropFromOutside}
	>
		{#if title}
			<Tooltip text={title} anchorStyle={{ flex: 1, minWidth: 0, alignSelf: "stretch" }}>
				{@render labelButton()}
			</Tooltip>
		{:else}
			{@render labelButton()}
		{/if}

		{#if onEdit}
			<IconButton icon={pencilSimple} size={12} title={editLabel} dimmed={!isHovered} onpress={onEdit} />
		{/if}
		{#if onRemove}
			<IconButton icon={x} size={12} title={removeLabel} dimmed={!isHovered} onpress={onRemove} />
		{/if}
	</View>
{/snippet}

<DropTarget accept={acceptsPaths} ondrop={({ data }) => dropFromWindow(data)}>
	{#snippet children({ over })}
		{@render row(over)}
	{/snippet}
</DropTarget>
