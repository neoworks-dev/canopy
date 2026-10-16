<!--
	The menu of commands that opens at a point, for a selection or a folder's
	background. The library's ContextMenu opens on its own right press and cannot
	be opened at a given point, so this places a Popover and a Menu at x and y.
	Groups are separated by a line; a command that cannot run is shown dimmed.
-->
<script lang="ts">
	import { Menu, Popover, type MenuItem as LibraryMenuItem } from "@neoworks/svelte-gpui/ui";
	import type { MenuItem } from "../lib/commands";
	import { floatingPanel } from "./floatingPanel";

	const menuWidth = 224;

	let {
		x,
		y,
		groups,
		onSelect,
		onClose,
	}: {
		x: number;
		y: number;
		groups: MenuItem[][];
		onSelect: (id: string) => void;
		onClose: () => void;
	} = $props();

	let isOpen = $state(true);

	const items = $derived(withSeparators(groups));

	function withSeparators(commandGroups: MenuItem[][]): LibraryMenuItem[] {
		const rows: LibraryMenuItem[] = [];
		commandGroups.forEach((group, index) => {
			if (index > 0) {
				rows.push({ separator: true });
			}
			rows.push(...group.map(rowFor));
		});
		return rows;
	}

	function rowFor(command: MenuItem): LibraryMenuItem {
		return {
			label: command.title,
			icon: command.icon,
			shortcut: command.shortcut,
			disabled: !command.enabled,
			onselect: () => onSelect(command.id),
		};
	}
</script>

<Popover
	bind:open={isOpen}
	at={[x, y]}
	side="bottom"
	align="start"
	offset={0}
	focusPanel={false}
	style={{ ...floatingPanel, padding: 0, gap: 0 }}
	onclose={onClose}
>
	{#snippet children({ close })}
		<Menu {items} width={menuWidth} onclose={() => close("select")} />
	{/snippet}
</Popover>
