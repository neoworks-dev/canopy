<script lang="ts">
	import { ripple } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import XIcon from "phosphor-svelte/lib/XIcon";
	import { dragSourceOf, dropOperation, readDraggedPaths, type DropOperation } from "./dragDrop";

	// ListRow from @neoworks-dev/ui is a content row with its own padding scale
	// and no active state, which is a size and a behaviour a navigation rail does
	// not want. This uses the same tokens at rail proportions.
	let {
		icon: ItemIcon,
		label,
		detail = "",
		title = "",
		active = false,
		onSelect,
		onRemove = undefined,
		removeLabel = "",
		onDropPaths = undefined,
	}: {
		icon: IconComponent;
		label: string;
		/** A short trailing value, such as the free space left on a volume. */
		detail?: string;
		title?: string;
		active?: boolean;
		onSelect: () => void;
		/** Present on the items that can be taken out of the sidebar. */
		onRemove?: () => void;
		removeLabel?: string;
		/** Present on the items a drag can be dropped on. */
		onDropPaths?: (sources: string[], operation: DropOperation) => void;
	} = $props();

	let isDropTarget = $state(false);

	// Cancelling both dragenter and dragover is what makes the item a drop target:
	// the webview does not settle for the second on its own.
	function trackDragOver(event: DragEvent) {
		if (!onDropPaths || !event.dataTransfer) {
			return;
		}
		event.preventDefault();
		isDropTarget = true;
		event.dataTransfer.dropEffect = dropOperation(dragSourceOf(event.dataTransfer), event);
	}

	function applyDrop(event: DragEvent) {
		isDropTarget = false;
		if (!onDropPaths || !event.dataTransfer) {
			return;
		}
		event.preventDefault();
		const transfer = event.dataTransfer;
		onDropPaths(readDraggedPaths(transfer), dropOperation(dragSourceOf(transfer), event));
	}

	function iconWeight(isActive: boolean): "fill" | "regular" {
		if (isActive) {
			return "fill";
		}
		return "regular";
	}

	function currentMarker(isActive: boolean): "page" | undefined {
		if (isActive) {
			return "page";
		}
		return undefined;
	}
</script>

<div
	role="group"
	ondragenter={trackDragOver}
	ondragover={trackDragOver}
	ondragleave={() => (isDropTarget = false)}
	ondrop={applyDrop}
	class="group/item flex items-center rounded-lg"
	class:bg-raised={active}
	class:ring-1={isDropTarget}
	class:ring-inset={isDropTarget}
	class:ring-action={isDropTarget}
>
	<button
		type="button"
		use:ripple
		onclick={onSelect}
		aria-current={currentMarker(active)}
		title={title}
		class="flex min-w-0 flex-1 items-center gap-2 rounded-lg p-2 text-left text-sm transition duration-fast"
		class:font-medium={active}
		class:text-default={active}
		class:text-muted={!active}
		class:hover:bg-hover={!active}
		class:hover:text-default={!active}
	>
		<ItemIcon size={16} weight={iconWeight(active)} class="shrink-0" />
		<span class="min-w-0 flex-1 truncate">{label}</span>
		{#if detail}
			<span class="shrink-0 text-2xs text-faint tabular-nums">{detail}</span>
		{/if}
	</button>

	{#if onRemove}
		<button
			type="button"
			onclick={onRemove}
			aria-label={removeLabel}
			class="mr-1 shrink-0 rounded-md p-1 text-faint opacity-0 transition duration-fast group-hover/item:opacity-100 hover:bg-hover hover:text-default focus-visible:opacity-100"
		>
			<XIcon size={12} />
		</button>
	{/if}
</div>
