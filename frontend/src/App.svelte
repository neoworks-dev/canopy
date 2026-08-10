<script lang="ts">
	import { onMount } from "svelte";
	import { Button, Card, ListRow, SectionHeader, LoadingSpinner } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import ArrowUpIcon from "phosphor-svelte/lib/ArrowUpIcon";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import EyeIcon from "phosphor-svelte/lib/EyeIcon";
	import EyeSlashIcon from "phosphor-svelte/lib/EyeSlashIcon";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import { ListDirectory } from "../wailsjs/go/main/FileService";
	import type { filesystem } from "../wailsjs/go/models";
	import EntryIcon from "./lib/EntryIcon.svelte";
	import PathBreadcrumb from "./lib/PathBreadcrumb.svelte";
	import { formatFileSize, formatModifiedTime } from "./lib/format";

	let listing = $state<filesystem.Listing | null>(null);
	let errorMessage = $state("");
	let isLoading = $state(false);
	let showHidden = $state(false);

	const currentPath = $derived(pathOf(listing));
	const canGoUp = $derived(Boolean(listing?.parentPath));
	const visibleEntries = $derived.by(collectVisibleEntries);

	onMount(() => {
		navigateTo("");
	});

	async function navigateTo(path: string) {
		isLoading = true;
		errorMessage = "";
		try {
			listing = await ListDirectory(path);
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
	}

	function collectVisibleEntries(): filesystem.Entry[] {
		if (!listing) {
			return [];
		}
		if (showHidden) {
			return listing.entries;
		}
		return listing.entries.filter((entry) => !entry.name.startsWith("."));
	}

	function openEntry(entry: filesystem.Entry) {
		if (!entry.isDir) {
			return;
		}
		navigateTo(entry.path);
	}

	function goUp() {
		if (!listing?.parentPath) {
			return;
		}
		navigateTo(listing.parentPath);
	}

	function toggleHidden() {
		showHidden = !showHidden;
	}

	function hiddenToggleIcon(): IconComponent {
		if (showHidden) {
			return EyeIcon;
		}
		return EyeSlashIcon;
	}

	function entrySubtitle(entry: filesystem.Entry): string {
		const modified = formatModifiedTime(entry.modTime);
		if (entry.isDir) {
			return modified;
		}
		return `${formatFileSize(entry.size)} · ${modified}`;
	}

	function pathOf(current: filesystem.Listing | null): string {
		if (!current) {
			return "";
		}
		return current.path;
	}

	function describeError(error: unknown): string {
		if (error instanceof Error) {
			return error.message;
		}
		return String(error);
	}

	function itemCountLabel(count: number): string {
		if (count === 1) {
			return "1 item";
		}
		return `${count} items`;
	}
</script>

<div class="flex h-full flex-col bg-canvas">
	<header class="flex items-center gap-2 border-b border-line px-4 py-3">
		<Button size="sm" variant="ghost" icon={ArrowUpIcon} disabled={!canGoUp} onclick={goUp} />
		<Button size="sm" variant="ghost" icon={HouseIcon} onclick={() => navigateTo("")} />

		<PathBreadcrumb path={currentPath} onNavigate={navigateTo} />

		<Button
			size="sm"
			variant="ghost"
			icon={hiddenToggleIcon()}
			onclick={toggleHidden}
		/>
	</header>

	<main class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
		{#if errorMessage}
			<Card>
				<div class="flex items-center gap-3">
					<WarningIcon size={20} class="shrink-0 text-red" />
					<div class="min-w-0">
						<p class="text-sm font-medium text-default">Could not open this folder</p>
						<p class="truncate text-xs text-dim">{errorMessage}</p>
					</div>
				</div>
			</Card>
		{:else if isLoading}
			<div class="flex justify-center py-12">
				<LoadingSpinner />
			</div>
		{:else}
			<SectionHeader title={itemCountLabel(visibleEntries.length)} />
			<div class="mt-2 flex flex-col gap-1">
				{#each visibleEntries as entry (entry.path)}
					<ListRow
						title={entry.name}
						subtitle={entrySubtitle(entry)}
						chevron={entry.isDir}
						onclick={() => openEntry(entry)}
					>
						{#snippet leading()}
							<EntryIcon {entry} />
						{/snippet}
					</ListRow>
				{/each}
			</div>
		{/if}
	</main>
</div>
