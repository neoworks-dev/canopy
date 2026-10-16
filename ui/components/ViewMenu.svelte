<!--
	The view options behind the sliders button: how the listing is laid out, which
	columns it shows, the details pane, and the sort grouping. The design system
	has no popover or menu, so this is the library's Popover holding a raised card.
-->
<script lang="ts">
	import { Button as PlainButton, Icon, Popover, Text, View, merge } from "@neoworks/svelte-gpui/ui";
	import { columns as columnsIcon, list, slidersHorizontal, squaresFour } from "../icons";
	import { availableColumns, type ColumnId } from "../lib/columns";
	import type { ViewMode } from "../lib/settings";
	import { colors, duration, radius, space, textSize } from "../theme";
	import Button from "./design/Button.svelte";
	import CheckRow from "./CheckRow.svelte";
		import { floatingPanel } from "./floatingPanel";

	const panelWidth = 192;

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

	const viewModes: { id: ViewMode; label: string; icon: string }[] = [
		{ id: "rows", label: "List", icon: list },
		{ id: "grid", label: "Grid", icon: squaresFour },
		{ id: "columns", label: "Columns", icon: columnsIcon },
	];

	let isOpen = $state(false);

	function toggleColumn(id: ColumnId) {
		if (visibleIds.includes(id)) {
			onChange(visibleIds.filter((visibleId) => visibleId !== id));
			return;
		}
		onChange([...visibleIds, id]);
	}

	function modeLook(isCurrent: boolean) {
		return merge(
			{
				direction: "row",
				align: "center",
				gap: space[2],
				padding: space[2],
				radius: radius.md,
				fontSize: textSize.sm,
				color: colors.muted,
				transition: duration.fast,
			},
			isCurrent && { background: colors.hover, color: colors.text },
		);
	}
</script>

{#snippet heading(title: string)}
	<Text style={{ paddingX: space[1], fontSize: textSize["2xs"], uppercase: true, color: colors.faint }}>
		{title}
	</Text>
{/snippet}

<Popover bind:open={isOpen} side="bottom" align="end" offset={space[2]} style={{ ...floatingPanel, width: panelWidth }}>
	{#snippet trigger({ toggle })}
		<Button size="sm" variant="ghost" icon={slidersHorizontal} onpress={toggle} />
	{/snippet}

	{@render heading("View")}
	{#each viewModes as mode (mode.id)}
		<PlainButton
			style={modeLook(viewMode === mode.id)}
			hover={{ background: colors.hover, color: colors.text }}
			onpress={() => onViewModeChange(mode.id)}
		>
			<Icon svg={mode.icon} size={14} />
			<Text>{mode.label}</Text>
		</PlainButton>
	{/each}

	{@render heading("Columns")}
	{#each availableColumns as column (column.id)}
		<CheckRow size="sm" label={column.label} checked={visibleIds.includes(column.id)} onchange={() => toggleColumn(column.id)} />
	{/each}

	{@render heading("Details")}
	<CheckRow size="sm" label="Show the pane" checked={showDetails} onchange={(shown) => onShowDetailsChange(shown)} />

	{@render heading("Sorting")}
	<CheckRow
		size="sm"
		label="Folders first"
		checked={directoriesFirst}
		onchange={(first) => onDirectoriesFirstChange(first)}
	/>
</Popover>
