<script lang="ts">
	import { Card } from "@neoworks-dev/ui";
	import type { ActionContext, ActionDefinition, ActionId } from "./actions";

	// @neoworks-dev/ui has no menu: Select picks one value from a list and NavBar
	// is a horizontal href tab bar, so neither carries grouped, disabled or
	// shortcut-labelled commands. This composes the shared Card instead.
	let {
		x,
		y,
		groups,
		context,
		onSelect,
		onClose,
	}: {
		x: number;
		y: number;
		groups: ActionDefinition[][];
		context: ActionContext;
		onSelect: (id: ActionId) => void;
		onClose: () => void;
	} = $props();

	const menuWidth = 224;
	const estimatedItemHeight = 32;
	const viewportMargin = 8;

	let menu = $state<HTMLDivElement>();
	let activeIndex = $state(-1);

	const items = $derived(groups.flat());
	const enabledIndexes = $derived(collectEnabledIndexes());
	const position = $derived(clampToViewport());

	$effect(() => {
		menu?.focus();
	});

	function collectEnabledIndexes(): number[] {
		const indexes: number[] = [];
		items.forEach((item, index) => {
			if (item.isEnabled(context)) {
				indexes.push(index);
			}
		});
		return indexes;
	}

	// Opening near an edge must not push the menu off screen; the height is
	// estimated from the item count because the panel is measured after paint.
	function clampToViewport(): { left: number; top: number } {
		const height = items.length * estimatedItemHeight + viewportMargin * 2;
		return {
			left: Math.min(x, window.innerWidth - menuWidth - viewportMargin),
			top: Math.min(y, Math.max(viewportMargin, window.innerHeight - height)),
		};
	}

	function closeOnOutsidePointer(event: PointerEvent) {
		if (menu?.contains(event.target as Node)) {
			return;
		}
		onClose();
	}

	function choose(item: ActionDefinition) {
		if (!item.isEnabled(context)) {
			return;
		}
		onSelect(item.id);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			chooseActive();
			return;
		}
		moveActive(event);
	}

	function chooseActive() {
		if (activeIndex < 0) {
			return;
		}
		choose(items[activeIndex]);
	}

	function moveActive(event: KeyboardEvent) {
		const step = arrowStep(event.key);
		if (step === 0) {
			return;
		}
		event.preventDefault();
		activeIndex = neighbourIndex(step);
	}

	function neighbourIndex(step: number): number {
		if (enabledIndexes.length === 0) {
			return -1;
		}
		const position = enabledIndexes.indexOf(activeIndex);
		const next = (position + step + enabledIndexes.length) % enabledIndexes.length;
		return enabledIndexes[next];
	}

	function arrowStep(key: string): number {
		if (key === "ArrowDown") {
			return 1;
		}
		if (key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

</script>

<svelte:window onpointerdown={closeOnOutsidePointer} onresize={onClose} />

<div
	bind:this={menu}
	role="menu"
	tabindex="-1"
	aria-label="Actions"
	onkeydown={handleKeydown}
	oncontextmenu={(event) => event.preventDefault()}
	style="left: {position.left}px; top: {position.top}px; width: {menuWidth}px"
	class="fixed z-50 focus:outline-none"
>
	<Card surface="raised" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
		{#each groups as group, groupIndex (groupIndex)}
			{#if groupIndex > 0}
				<div class="h-px bg-line-faint"></div>
			{/if}
			{#each group as item (item.id)}
				{@const index = items.indexOf(item)}
				{@const ItemIcon = item.icon}
				{@const isEnabled = item.isEnabled(context)}
				<button
					type="button"
					role="menuitem"
					disabled={!isEnabled}
					onclick={() => choose(item)}
					onmouseenter={() => (activeIndex = index)}
					class="flex items-center gap-2 rounded-md p-2 text-left text-sm transition duration-fast"
					class:text-faint={!isEnabled}
					class:bg-hover={isEnabled && index === activeIndex}
					class:text-default={isEnabled && index === activeIndex}
					class:text-muted={isEnabled && index !== activeIndex}
					class:hover:bg-hover={isEnabled}
					class:hover:text-default={isEnabled}
				>
					<ItemIcon size={16} class="shrink-0" />
					<span class="min-w-0 flex-1 truncate">{item.label}</span>
					{#if item.shortcut}
						<span class="shrink-0 text-2xs text-faint">{item.shortcut}</span>
					{/if}
				</button>
			{/each}
		{/each}
	</Card>
</div>
