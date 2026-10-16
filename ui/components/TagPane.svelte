<!--
	The files carrying one tag. Tags live on the files themselves, so finding
	them means looking through the home folder, which takes a moment. It fills
	the space it is given and scrolls its results.
-->
<script lang="ts">
	import { List, Text, View } from "@neoworks/svelte-gpui/ui";
	import { FilesWithTag } from "../backend/FileService";
	import type { filesystem } from "../backend/models";
	import { describeError } from "../lib/errors";
	import { colors, space, textSize } from "../theme";
	import EntryHitRow from "./EntryHitRow.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";
	import SectionHeader from "./design/SectionHeader.svelte";

	let {
		name,
		onOpen,
		onReveal,
	}: {
		name: string;
		onOpen: (entry: filesystem.Entry) => void;
		onReveal: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state<filesystem.Entry[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);

	// Guards against a slow walk for one tag landing after the user has already
	// picked another.
	let latestSearch = 0;

	$effect(() => {
		search(name);
	});

	async function search(tag: string) {
		const sequence = ++latestSearch;
		isLoading = true;
		errorMessage = "";
		try {
			applyResults(sequence, await FilesWithTag(tag), "");
		} catch (error) {
			applyResults(sequence, [], describeError(error));
		}
	}

	function applyResults(sequence: number, found: filesystem.Entry[], message: string) {
		if (sequence !== latestSearch) {
			return;
		}
		entries = found;
		errorMessage = message;
		isLoading = false;
	}
</script>

<View style={{ flex: 1, minHeight: 0, gap: space[2] }}>
	<View style={{ direction: "row", align: "center", gap: space[2] }}>
		<SectionHeader title={name} />
		<View style={{ flex: 1 }} />
		<Text style={{ paddingBottom: space[2], fontSize: textSize.xs, color: colors.faint }}>
			{entries.length} tagged
		</Text>
	</View>

	{#if errorMessage}
		<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
	{/if}

	{#if isLoading}
		<View style={{ align: "center", gap: space[2], padding: space[2] }}>
			<LoadingSpinner />
			<Text style={{ fontSize: textSize["2xs"], color: colors.faint }}>
				Tags live on the files themselves, so this looks through your home folder
			</Text>
		</View>
	{:else if entries.length === 0}
		<Text style={{ fontSize: textSize.sm, color: colors.dim }}>Nothing in your home folder carries this tag.</Text>
	{:else}
		<List items={entries} key={(entry) => entry.path} style={{ flex: 1, minHeight: 0 }}>
			{#snippet row(entry)}
				<View style={{ paddingBottom: space[2] }}>
					<EntryHitRow {entry} {onOpen} {onReveal} />
				</View>
			{/snippet}
		</List>
	{/if}
</View>
