<!--
	The tabs of one pane group, with a button for a new tab and one to split the
	view. The design system has no tab bar, so this is built from its tokens at
	the proportions a pane header needs. The rail is the recessed surface the
	pane sits on: the selected tab carries the pane's own background down to the
	listing, so the two read as one panel.
-->
<script lang="ts">
	import { Button, Icon, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
	import { columnsBold, plusBold, xBold } from "../icons";
	import { tabTitle, type PaneGroup, type Tab } from "../lib/tabs";
	import { colors, duration, radius, space, textSize } from "../theme";

	let {
		group,
		isActive,
		canSplit,
		onSelect,
		onClose,
		onOpen,
		onSplit,
	}: {
		group: PaneGroup;
		isActive: boolean;
		canSplit: boolean;
		onSelect: (tabId: string) => void;
		onClose: (tabId: string) => void;
		onOpen: () => void;
		onSplit: () => void;
	} = $props();

	// The tab the pointer is over shows its close button.
	let hoveredTabId = $state("");

	function titleColor(isSelected: boolean): string {
		if (isSelected && isActive) {
			return colors.text;
		}
		if (isSelected) {
			return colors.dim;
		}
		return colors.muted;
	}

	function tabBackground(isSelected: boolean): string {
		if (isSelected) {
			return colors.elevated;
		}
		return "transparent";
	}

	function tabHover(isSelected: boolean) {
		if (isSelected) {
			return {};
		}
		return { background: colors.hover };
	}

	function hover(tab: Tab, isHovered: boolean) {
		if (isHovered) {
			hoveredTabId = tab.id;
			return;
		}
		if (hoveredTabId === tab.id) {
			hoveredTabId = "";
		}
	}

	const railButton = {
		shrink: 0,
		align: "center",
		justify: "center",
		paddingX: space[1.5],
		color: colors.faint,
		transition: duration.fast,
	} as const;
</script>

<View style={{ direction: "row", shrink: 0, align: "stretch", background: colors.raised, fontSize: textSize.xs }}>
	<Scroll horizontal style={{ minWidth: 0, shrink: 1 }}>
		<View style={{ direction: "row", align: "stretch" }}>
			{#each group.tabs as tab (tab.id)}
				{@const isSelected = tab.id === group.activeTabId}
				<View
					style={{
						direction: "row",
						align: "center",
						background: tabBackground(isSelected),
						transition: duration.fast,
					}}
					hover={tabHover(isSelected)}
					onhover={(isHovered) => hover(tab, isHovered)}
				>
					<Button
						style={{
							minWidth: 0,
							maxWidth: 160,
							paddingX: space[3],
							paddingY: space[2],
							color: titleColor(isSelected),
							transition: duration.fast,
						}}
						hover={{ color: colors.text }}
						onpress={() => onSelect(tab.id)}
					>
						<Text style={{ truncate: true }}>{tabTitle(tab)}</Text>
					</Button>
					{#if hoveredTabId === tab.id}
						<Button
							style={{ shrink: 0, marginRight: space[2], padding: space[0.5], radius: radius.xs, color: colors.faint }}
							hover={{ background: colors.hover, color: colors.text }}
							onpress={() => onClose(tab.id)}
						>
							<Icon svg={xBold} size={10} />
						</Button>
					{/if}
				</View>
			{/each}

			<Button style={railButton} hover={{ background: colors.hover, color: colors.text }} onpress={onOpen}>
				<Icon svg={plusBold} size={12} />
			</Button>
		</View>
	</Scroll>

	<View style={{ flex: 1 }} />

	{#if canSplit}
		<Button style={railButton} hover={{ background: colors.hover, color: colors.text }} onpress={onSplit}>
			<Icon svg={columnsBold} size={12} />
		</Button>
	{/if}
</View>
