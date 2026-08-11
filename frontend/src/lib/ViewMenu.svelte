<script lang="ts">
	import { Button, Card, Checkbox } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import ListIcon from "phosphor-svelte/lib/ListIcon";
	import SlidersHorizontalIcon from "phosphor-svelte/lib/SlidersHorizontalIcon";
	import SquaresFourIcon from "phosphor-svelte/lib/SquaresFourIcon";
	import ColumnsIcon from "phosphor-svelte/lib/ColumnsIcon";
	import { availableColumns, type ColumnId } from "./columns";
	import type { ViewMode } from "./settings";

	// @neoworks-dev/ui has no popover or menu: Select picks one value and Tooltip
	// is hover-only, so the panel is a Card anchored to its trigger.
	let {
		viewMode,
		onViewModeChange,
		visibleIds,
		onChange,
		directoriesFirst,
		onDirectoriesFirstChange,
		showDetails,
		onShowDetailsChange,
	}: {
		viewMode: ViewMode;
		onViewModeChange: (mode: ViewMode) => void;
		visibleIds: ColumnId[];
		onChange: (ids: ColumnId[]) => void;
		directoriesFirst: boolean;
		onDirectoriesFirstChange: (directoriesFirst: boolean) => void;
		showDetails: boolean;
		onShowDetailsChange: (showDetails: boolean) => void;
	} = $props();

	const viewModes: { id: ViewMode; label: string; icon: IconComponent }[] = [
		{ id: "rows", label: "List", icon: ListIcon },
		{ id: "grid", label: "Grid", icon: SquaresFourIcon },
		{ id: "columns", label: "Columns", icon: ColumnsIcon },
	];

	let isOpen = $state(false);
	let container = $state<HTMLDivElement>();

	function toggleColumn(id: ColumnId) {
		if (visibleIds.includes(id)) {
			onChange(visibleIds.filter((visibleId) => visibleId !== id));
			return;
		}
		onChange([...visibleIds, id]);
	}

	function toggleMenu() {
		isOpen = !isOpen;
	}

	function closeOnOutsidePointer(event: PointerEvent) {
		if (!isOpen || !container) {
			return;
		}
		if (container.contains(event.target as Node)) {
			return;
		}
		isOpen = false;
	}
</script>

<svelte:window onpointerdown={closeOnOutsidePointer} />

<div class="relative shrink-0" bind:this={container}>
	<Button size="sm" variant="ghost" icon={SlidersHorizontalIcon} onclick={toggleMenu} />

	{#if isOpen}
		<div class="absolute top-full right-0 z-50 mt-2 w-48">
			<Card surface="raised" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
				<p class="px-1 text-2xs tracking-caps text-faint uppercase">View</p>
				{#each viewModes as mode (mode.id)}
					{@const Icon = mode.icon}
					<button
						type="button"
						onclick={() => onViewModeChange(mode.id)}
						aria-pressed={viewMode === mode.id}
						class="flex items-center gap-2 rounded-md p-2 text-left text-sm transition duration-fast"
						class:bg-hover={viewMode === mode.id}
						class:text-default={viewMode === mode.id}
						class:text-muted={viewMode !== mode.id}
						class:hover:bg-hover={viewMode !== mode.id}
						class:hover:text-default={viewMode !== mode.id}
					>
						<Icon size={14} />
						<span>{mode.label}</span>
					</button>
				{/each}

				<p class="px-1 text-2xs tracking-caps text-faint uppercase">Columns</p>
				{#each availableColumns as column (column.id)}
					<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
						<Checkbox
							size="sm"
							checked={visibleIds.includes(column.id)}
							onchange={() => toggleColumn(column.id)}
						/>
						<span class="text-sm text-default">{column.label}</span>
					</label>
				{/each}

				<p class="px-1 text-2xs tracking-caps text-faint uppercase">Details</p>
				<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
					<Checkbox
						size="sm"
						checked={showDetails}
						onchange={() => onShowDetailsChange(!showDetails)}
					/>
					<span class="text-sm text-default">Show the pane</span>
				</label>

				<p class="px-1 text-2xs tracking-caps text-faint uppercase">Sorting</p>
				<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
					<Checkbox
						size="sm"
						checked={directoriesFirst}
						onchange={() => onDirectoriesFirstChange(!directoriesFirst)}
					/>
					<span class="text-sm text-default">Folders first</span>
				</label>
			</Card>
		</div>
	{/if}
</div>
