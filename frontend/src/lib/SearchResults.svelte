<script lang="ts">
	import { ListRow, SectionHeader } from "@neoworks-dev/ui";
	import type { proto } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { formatFileSize, formatModifiedUnixTime, parentDirectoryOf } from "./format";

	let {
		hits,
		truncated,
		onOpen,
	}: {
		hits: proto.Hit[];
		truncated: boolean;
		onOpen: (hit: proto.Hit) => void;
	} = $props();

	const countLabel = $derived(describeCount(hits.length, truncated));

	function describeCount(count: number, isTruncated: boolean): string {
		if (isTruncated) {
			return `First ${count} matches`;
		}
		if (count === 1) {
			return "1 match";
		}
		return `${count} matches`;
	}

	// A hit can be anywhere in the indexed roots, so its location matters more
	// than it does in a directory listing.
	function hitSubtitle(hit: proto.Hit): string {
		const location = parentDirectoryOf(hit.path);
		if (hit.isDir) {
			return location;
		}
		return `${location} · ${formatFileSize(hit.size)} · ${formatModifiedUnixTime(hit.modTimeUnix)}`;
	}
</script>

<SectionHeader title={countLabel} />
<div class="mt-2 flex flex-col gap-1">
	{#each hits as hit (hit.path)}
		<ListRow
			title={hit.name}
			subtitle={hitSubtitle(hit)}
			chevron={hit.isDir}
			onclick={() => onOpen(hit)}
		>
			{#snippet leading()}
				<EntryIcon entry={{ name: hit.name, isDir: hit.isDir }} />
			{/snippet}
		</ListRow>
	{/each}
</div>
