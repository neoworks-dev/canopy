<script lang="ts">
	import CaretDownIcon from "phosphor-svelte/lib/CaretDownIcon";
	import CaretUpIcon from "phosphor-svelte/lib/CaretUpIcon";
	import { sortDirectionOf, type SortKey, type SortOrder } from "./sorting";

	// One column header. A column the backend cannot sort by renders as the plain
	// label it was before, rather than as a control that does nothing.
	let {
		label,
		sortKey,
		widthClass,
		valueClass,
		order,
		onSort,
	}: {
		label: string;
		sortKey: SortKey | "";
		widthClass: string;
		valueClass: string;
		order: SortOrder;
		onSort: (key: SortKey) => void;
	} = $props();

	const direction = $derived(directionOf());

	function directionOf(): "ascending" | "descending" | "" {
		if (!sortKey) {
			return "";
		}
		return sortDirectionOf(order, sortKey);
	}

	function sortByThisColumn() {
		if (!sortKey) {
			return;
		}
		onSort(sortKey);
	}
</script>

<span class="shrink-0 {widthClass} {valueClass}">
	{#if sortKey}
		<button
			type="button"
			aria-label="Sort by {label}"
			onclick={sortByThisColumn}
			class="inline-flex max-w-full items-center gap-1 rounded-sm hover:text-default focus-visible:ring-1 focus-visible:ring-line-faint focus-visible:outline-none"
			class:text-default={direction !== ""}
		>
			<span class="truncate">{label}</span>
			{#if direction === "ascending"}
				<CaretUpIcon size={10} class="shrink-0" />
			{/if}
			{#if direction === "descending"}
				<CaretDownIcon size={10} class="shrink-0" />
			{/if}
		</button>
	{:else}
		<span class="truncate">{label}</span>
	{/if}
</span>
