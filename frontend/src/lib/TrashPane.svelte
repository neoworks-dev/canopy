<script lang="ts">
	import { Button, LoadingSpinner, SectionHeader } from "@neoworks-dev/ui";
	import ArrowUUpLeftIcon from "phosphor-svelte/lib/ArrowUUpLeftIcon";
	import TrashIcon from "phosphor-svelte/lib/TrashIcon";
	import { EmptyTrash, ListTrash, RestoreFromTrash } from "../../wailsjs/go/main/FileService";
	import type { fileops } from "../../wailsjs/go/models";
	import ConfirmDialog from "./ConfirmDialog.svelte";
	import { describeError } from "./errors";
	import { formatFileSize, formatModifiedTime } from "./format";

	let { onRestored }: { onRestored: (path: string) => void } = $props();

	let items = $state<fileops.TrashedItem[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);
	let isConfirmingEmpty = $state(false);

	$effect(() => {
		reload();
	});

	async function reload() {
		isLoading = true;
		errorMessage = "";
		try {
			items = await ListTrash();
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
	}

	async function restore(item: fileops.TrashedItem) {
		try {
			const restoredPath = await RestoreFromTrash(item.trashPath);
			onRestored(restoredPath);
		} catch (error) {
			errorMessage = describeError(error);
		}
		reload();
	}

	async function confirmEmpty() {
		isConfirmingEmpty = false;
		try {
			await EmptyTrash();
		} catch (error) {
			errorMessage = describeError(error);
		}
		reload();
	}

	function sizeLabel(item: fileops.TrashedItem): string {
		if (item.isDir) {
			return "—";
		}
		return formatFileSize(item.size);
	}
</script>

<div class="flex items-center gap-2">
	<SectionHeader title="Trash" />
	<div class="flex-1"></div>
	<Button
		size="sm"
		variant="ghost"
		icon={TrashIcon}
		disabled={items.length === 0}
		onclick={() => (isConfirmingEmpty = true)}
	>
		Empty
	</Button>
</div>

{#if errorMessage}
	<p class="text-xs text-red">{errorMessage}</p>
{/if}

{#if isLoading}
	<div class="flex justify-center p-2"><LoadingSpinner /></div>
{:else if items.length === 0}
	<p class="text-sm text-dim">The trash is empty.</p>
{:else}
	<div class="flex min-w-0 flex-col gap-2">
		{#each items as item (item.trashPath)}
			<div class="flex items-center gap-2 rounded-lg p-2 hover:bg-hover">
				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-medium text-default">{item.name}</p>
					<p class="truncate text-2xs text-faint">{item.originalPath}</p>
				</div>
				<span class="shrink-0 text-xs text-dim tabular-nums">{sizeLabel(item)}</span>
				<span class="shrink-0 text-xs text-dim">{formatModifiedTime(item.deletedAt)}</span>
				<Button size="sm" variant="ghost" icon={ArrowUUpLeftIcon} onclick={() => restore(item)} />
			</div>
		{/each}
	</div>
{/if}

{#if isConfirmingEmpty}
	<ConfirmDialog
		title="Empty the trash?"
		message="Every item in the trash is deleted permanently. This cannot be undone."
		confirmLabel="Empty trash"
		onConfirm={confirmEmpty}
		onCancel={() => (isConfirmingEmpty = false)}
	/>
{/if}
