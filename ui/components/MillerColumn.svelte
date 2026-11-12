<!--
	A folder read on its own and drawn as Yazi draws the columns beside the one
	in focus: the parent, with the folder on screen marked as hovered, or the
	preview of a hovered folder. It takes no keys; a press opens.
-->
<script lang="ts">
	import { List, Text, View, type Style } from "@neoworks/svelte-gpui/ui";
	import { ListDirectory } from "../backend/FileService";
	import type { filesystem } from "../backend/models";
	import { entryRowHeight, type EntryState } from "../lib/entryStyles";
	import { leadingChild } from "../lib/millerColumns";
	import type { SortOrder } from "../lib/sorting";
	import { colors, space, textSize } from "../theme";
	import EntryLine from "./EntryLine.svelte";

	let {
		path,
		currentPath = "",
		sortOrder,
		showHidden,
		share = 1,
		isDivided = true,
		onOpen,
	}: {
		path: string;
		/** The folder the pane is showing, which this column may lead towards. */
		currentPath?: string;
		sortOrder: SortOrder;
		showHidden: boolean;
		/** How much of the row's width it takes, against its siblings' shares. */
		share?: number;
		/** Set off from the column after it by a line, as the parent is from the listing. */
		isDivided?: boolean;
		onOpen: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state<filesystem.Entry[]>([]);
	let isLoading = $state(true);
	let hasFailed = $state(false);

	// Guards against a slow read of one folder landing after the chain moved on.
	let latestRead = 0;

	const openChild = $derived(leadingChild(path, currentPath));
	const visibleEntries = $derived(withoutHidden(entries, showHidden));

	let list = $state<ReturnType<typeof List<filesystem.Entry>>>();

	$effect(() => {
		read(path, sortOrder);
	});

	// The folder this column leads to is kept in view, as Yazi keeps the parent's hovered line.
	$effect(() => {
		const index = visibleEntries.findIndex((entry) => entry.path === openChild);
		if (index < 0) {
			return;
		}
		list?.scrollToIndex(index, { align: "center" });
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

	function stateOf(entry: filesystem.Entry): EntryState {
		return {
			isSelected: false,
			isFocused: entry.path === openChild,
			isDropOver: false,
			isCut: false,
			isCopied: false,
			isMuted: false,
		};
	}

	// A column beside the listing is set off by a line and some room; a preview stands alone.
	function dividerLook(): Style {
		if (!isDivided) {
			return {};
		}
		return { paddingRight: space[2], marginRight: space[2], borderRightWidth: 1, borderColor: colors.lineFaint };
	}

	function notice(): string {
		if (hasFailed) {
			return "Cannot read this folder";
		}
		if (!isLoading && visibleEntries.length === 0) {
			return "No items";
		}
		return "";
	}
</script>

<View style={{ flex: share, minWidth: 0, ...dividerLook() }}>
	{#if notice()}
		<Text style={{ padding: space[2], fontSize: textSize.xs, color: colors.faint }}>{notice()}</Text>
	{:else}
		<List bind:this={list} items={visibleEntries} rowHeight={entryRowHeight} key={(entry) => entry.path} style={{ flex: 1, minHeight: 0 }}>
			{#snippet row(entry)}
				<EntryLine {entry} state={stateOf(entry)} onpress={() => onOpen(entry)} />
			{/snippet}
		</List>
	{/if}
</View>
