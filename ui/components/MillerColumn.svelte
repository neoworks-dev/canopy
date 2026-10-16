<!--
	One column of a Miller chain: the entries of one folder, with the child that
	leads on towards the folder on screen marked.
-->
<script lang="ts">
	import { Button, Icon, List, Text, View } from "@neoworks/svelte-gpui/ui";
	import { ListDirectory } from "../backend/FileService";
	import type { filesystem } from "../backend/models";
	import { caretRight } from "../icons";
	import { entryRowHeight } from "../lib/entryStyles";
	import { leadingChild } from "../lib/millerColumns";
	import type { SortOrder } from "../lib/sorting";
	import { colors, duration, radius, space, textSize } from "../theme";
	import EntryIcon from "./EntryIcon.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";

	const columnWidth = 224;

	let {
		path,
		currentPath,
		sortOrder,
		showHidden,
		onOpen,
	}: {
		path: string;
		/** The folder the pane is showing, which this column may lead towards. */
		currentPath: string;
		sortOrder: SortOrder;
		showHidden: boolean;
		onOpen: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state<filesystem.Entry[]>([]);
	let isLoading = $state(true);
	let hasFailed = $state(false);

	// Guards against a slow read of one folder landing after the chain moved on.
	let latestRead = 0;

	const openChild = $derived(leadingChild(path, currentPath));
	const visibleEntries = $derived(withoutHidden(entries, showHidden));

	$effect(() => {
		read(path, sortOrder);
	});

	async function read(wanted: string, order: SortOrder) {
		const sequence = ++latestRead;
		isLoading = true;
		try {
			applyListing(sequence, await ListDirectory(wanted, order));
		} catch {
			applyFailure(sequence);
		}
	}

	function applyListing(sequence: number, listing: filesystem.Listing) {
		if (sequence !== latestRead) {
			return;
		}
		entries = listing.entries;
		hasFailed = false;
		isLoading = false;
	}

	function applyFailure(sequence: number) {
		if (sequence !== latestRead) {
			return;
		}
		entries = [];
		hasFailed = true;
		isLoading = false;
	}

	function withoutHidden(listed: filesystem.Entry[], isShowingHidden: boolean): filesystem.Entry[] {
		if (isShowingHidden) {
			return listed;
		}
		return listed.filter((entry) => !entry.name.startsWith("."));
	}

	function nameColor(isOpen: boolean): string {
		if (isOpen) {
			return colors.text;
		}
		return colors.muted;
	}

	function rowBackground(isOpen: boolean): string {
		if (isOpen) {
			return colors.raised;
		}
		return "transparent";
	}

	function rowHover(isOpen: boolean) {
		if (isOpen) {
			return {};
		}
		return { background: colors.hover };
	}
</script>

<View
	style={{
		width: columnWidth,
		shrink: 0,
		paddingRight: space[2],
		borderRightWidth: 1,
		borderColor: colors.lineFaint,
	}}
>
	{#if isLoading}
		<View style={{ align: "center", padding: space[2] }}><LoadingSpinner size={14} /></View>
	{:else if hasFailed}
		<Text style={{ padding: space[2], fontSize: textSize["2xs"], color: colors.faint }}>
			This folder could not be read.
		</Text>
	{:else if visibleEntries.length === 0}
		<Text style={{ padding: space[2], fontSize: textSize["2xs"], color: colors.faint }}>Empty</Text>
	{:else}
		<List items={visibleEntries} rowHeight={entryRowHeight} key={(entry) => entry.path} style={{ flex: 1, minHeight: 0 }}>
			{#snippet row(entry)}
				{@const isOpen = entry.path === openChild}
				<Button
					style={{
						flex: 1,
						align: "center",
						gap: space[2],
						paddingX: space[2],
						radius: radius.md,
						background: rowBackground(isOpen),
						transition: duration.fast,
					}}
					hover={rowHover(isOpen)}
					onpress={() => onOpen(entry)}
				>
					<EntryIcon {entry} />
					<Text
						style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.sm, color: nameColor(isOpen) }}
					>
						{entry.name}
					</Text>
					{#if entry.isDir}
						<Icon svg={caretRight} size={10} color={colors.faint} style={{ shrink: 0 }} />
					{/if}
				</Button>
			{/snippet}
		</List>
	{/if}
</View>
