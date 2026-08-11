<script lang="ts">
	import { Button, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import CaretRightIcon from "phosphor-svelte/lib/CaretRightIcon";
	import FileIcon from "phosphor-svelte/lib/FileIcon";
	import FolderIcon from "phosphor-svelte/lib/FolderIcon";
	import { ListArchive } from "../../wailsjs/go/main/FileService";
	import {
		insideCrumbs,
		joinInside,
		parentInside,
		rowsInside,
		type ArchiveMember,
		type ArchiveRow,
	} from "./archive";
	import { describeError } from "./errors";
	import { formatFileSize, formatModifiedTime } from "./format";

	let {
		path,
		inside,
		onDescend,
		onExtract,
	}: {
		path: string;
		/** The level inside the archive being looked at, empty at the top. */
		inside: string;
		onDescend: (inside: string) => void;
		onExtract: () => void;
	} = $props();

	let members = $state<ArchiveMember[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(false);

	// Guards against a slow listing of an archive this pane has already left
	// landing after the one it is showing now.
	let latestRequest = 0;

	const rows = $derived(rowsInside(members, inside));
	const crumbs = $derived(insideCrumbs(inside));

	$effect(() => {
		read(path);
	});

	async function read(archivePath: string) {
		const request = ++latestRequest;
		isLoading = true;
		errorMessage = "";
		try {
			apply(request, await ListArchive(archivePath), "");
		} catch (error) {
			apply(request, [], describeError(error));
		}
	}

	function apply(request: number, listed: ArchiveMember[], message: string) {
		if (request !== latestRequest) {
			return;
		}
		members = listed;
		errorMessage = message;
		isLoading = false;
	}

	function open(row: ArchiveRow) {
		if (!row.isDir) {
			return;
		}
		onDescend(joinInside(inside, row.label));
	}

	function sizeLabel(row: ArchiveRow): string {
		if (row.isDir) {
			return "—";
		}
		return formatFileSize(row.size);
	}
</script>

<div class="flex items-center gap-2">
	<SectionHeader title="Archive" />
	<span class="min-w-0 flex-1 truncate text-xs text-faint">{path}</span>
	<Button size="sm" variant="ghost" onclick={onExtract}>Extract all</Button>
</div>

<div class="flex flex-wrap items-center gap-1 text-xs text-dim">
	<button type="button" class="rounded px-1 hover:bg-hover" onclick={() => onDescend("")}>
		Top
	</button>
	{#each crumbs as crumb (crumb.inside)}
		<CaretRightIcon size={12} class="text-faint" />
		<button
			type="button"
			class="rounded px-1 hover:bg-hover"
			onclick={() => onDescend(crumb.inside)}
		>
			{crumb.label}
		</button>
	{/each}
</div>

{#if errorMessage}
	<p class="text-xs text-red">{errorMessage}</p>
{:else if isLoading}
	<div class="flex justify-center p-2"><LoadingSpinner /></div>
{:else if rows.length === 0}
	<p class="text-sm text-dim">There is nothing in this archive.</p>
{:else}
	<div class="flex min-w-0 flex-col">
		{#if inside}
			<button
				type="button"
				onclick={() => onDescend(parentInside(inside))}
				class="flex items-center gap-2 rounded-lg p-2 text-left hover:bg-hover"
			>
				<FolderIcon size={20} class="shrink-0 text-accent" />
				<span class="text-sm text-dim">..</span>
			</button>
		{/if}
		{#each rows as row (row.label)}
			<button
				type="button"
				ondblclick={() => open(row)}
				class="flex items-center gap-2 rounded-lg p-2 text-left hover:bg-hover"
			>
				{#if row.isDir}
					<FolderIcon size={20} class="shrink-0 text-accent" />
				{:else}
					<FileIcon size={20} class="shrink-0 text-dim" />
				{/if}
				<span class="min-w-0 flex-1 truncate text-sm text-default">{row.label}</span>
				<span class="shrink-0 text-xs text-dim tabular-nums">{sizeLabel(row)}</span>
				<span class="shrink-0 text-xs text-dim">{formatModifiedTime(row.modTime)}</span>
			</button>
		{/each}
	</div>
{/if}
