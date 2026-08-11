<script lang="ts">
	import { Button, Card, Checkbox } from "@neoworks-dev/ui";
	import SlidersHorizontalIcon from "phosphor-svelte/lib/SlidersHorizontalIcon";
	import { availableColumns, type ColumnId } from "./columns";

	// @neoworks-dev/ui has no popover or menu: Select picks one value and Tooltip
	// is hover-only, so the panel is a Card anchored to its trigger.
	let {
		visibleIds,
		onChange,
		directoriesFirst,
		onDirectoriesFirstChange,
	}: {
		visibleIds: ColumnId[];
		onChange: (ids: ColumnId[]) => void;
		directoriesFirst: boolean;
		onDirectoriesFirstChange: (directoriesFirst: boolean) => void;
	} = $props();

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
	<Button
		size="sm"
		variant="ghost"
		icon={SlidersHorizontalIcon}
		onclick={toggleMenu}
	/>

	{#if isOpen}
		<div class="absolute top-full right-0 z-50 mt-2 w-48">
			<Card surface="raised" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
				<p class="px-1 text-2xs tracking-caps text-faint uppercase">Columns</p>
				{#each availableColumns as column (column.id)}
					<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
						<Checkbox
							checked={visibleIds.includes(column.id)}
							onchange={() => toggleColumn(column.id)}
						/>
						<span class="text-sm text-default">{column.label}</span>
					</label>
				{/each}

				<p class="px-1 text-2xs tracking-caps text-faint uppercase">Sorting</p>
				<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
					<Checkbox
						checked={directoriesFirst}
						onchange={() => onDirectoriesFirstChange(!directoriesFirst)}
					/>
					<span class="text-sm text-default">Folders first</span>
				</label>
			</Card>
		</div>
	{/if}
</div>
