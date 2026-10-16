<!--
	A saved search shown as a folder: the query is run again when the pane
	opens, so what it holds is whatever matches now rather than what it matched
	when it was saved. It fills the space it is given and scrolls its results.
-->
<script lang="ts">
	import { List, Text, View } from "@neoworks/svelte-gpui/ui";
	import { SearchEntries } from "../backend/IndexService";
	import type { filesystem } from "../backend/models";
	import { describeError } from "../lib/errors";
	import { searchDescription, type SavedSearch } from "../lib/savedSearches";
	import { colors, space, textSize } from "../theme";
	import EntryHitRow from "./EntryHitRow.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";
	import SectionHeader from "./design/SectionHeader.svelte";

	let {
		search,
		onOpen,
		onReveal,
	}: {
		search: SavedSearch;
		onOpen: (entry: filesystem.Entry) => void;
		onReveal: (entry: filesystem.Entry) => void;
	} = $props();

	let entries = $state.raw<filesystem.Entry[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);

	// Guards against a slow query landing after the user has opened another.
	let latestRun = 0;

	$effect(() => {
		run(search);
	});

	async function run(wanted: SavedSearch) {
		const sequence = ++latestRun;
		isLoading = true;
		errorMessage = "";
		try {
			applyResults(sequence, await SearchEntries(wanted.query, wanted.scope), "");
		} catch (error) {
			applyResults(sequence, [], describeError(error));
		}
	}

	function applyResults(sequence: number, found: filesystem.Entry[], message: string) {
		if (sequence !== latestRun) {
			return;
		}
		entries = found;
		errorMessage = message;
		isLoading = false;
	}
</script>

<View style={{ flex: 1, minHeight: 0, gap: space[2] }}>
	<View style={{ direction: "row", align: "center", gap: space[2] }}>
		<SectionHeader title={search.name} />
		<View style={{ flex: 1 }} />
		<Text style={{ paddingBottom: space[2], fontSize: textSize.xs, color: colors.faint }}>
			{entries.length} matching
		</Text>
	</View>

	<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>{searchDescription(search)}</Text>

	{#if errorMessage}
		<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
	{/if}

	{#if isLoading}
		<View style={{ align: "center", padding: space[2] }}>
			<LoadingSpinner />
		</View>
	{:else if entries.length === 0}
		<Text style={{ fontSize: textSize.sm, color: colors.dim }}>Nothing matches this search right now.</Text>
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
