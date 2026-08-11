<script lang="ts">
	import { Card, LoadingSpinner } from "@neoworks-dev/ui";
	import BookmarkSimpleIcon from "phosphor-svelte/lib/BookmarkSimpleIcon";
	import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
	import WarningIcon from "phosphor-svelte/lib/WarningIcon";
	import XIcon from "phosphor-svelte/lib/XIcon";
	import type { proto } from "../../wailsjs/go/models";
	import EntryIcon from "./EntryIcon.svelte";
	import { parentDirectoryOf } from "./format";
	import type { ThumbnailSource } from "./thumbnails";

	// The dropdown is where results live, so it holds enough of them to pick from
	// without covering the listing it is opening things in.
	const maxSuggestions = 10;

	// @neoworks-dev/ui has no text input yet; Select is a dropdown and its
	// internal search box is not exported. This mirrors that box's tokens so the
	// field still looks like the design system.
	let {
		value = $bindable(""),
		placeholder = "Search files",
		scope = "",
		hits = [],
		truncated = false,
		errorMessage = "",
		indexingLabel = "",
		isSearching = false,
		onOpenHit,
		onSave,
		onClose,
		input = $bindable<HTMLInputElement | undefined>(),
	}: {
		value?: string;
		placeholder?: string;
		/** The folder the search is confined to, or "" for every indexed root. */
		scope?: string;
		hits?: proto.Hit[];
		truncated?: boolean;
		errorMessage?: string;
		/** Said while the first crawl is still growing the result set. */
		indexingLabel?: string;
		isSearching?: boolean;
		onOpenHit: (hit: proto.Hit) => void;
		/** Keeps the query in the sidebar to be run again later. */
		onSave: () => void;
		onClose: () => void;
		/** Exposed so a shortcut elsewhere can put the caret in this field. */
		input?: HTMLInputElement;
	} = $props();

	let activeSuggestion = $state(-1);

	const suggestions = $derived(hits.slice(0, maxSuggestions));
	const isDropdownOpen = $derived(value.trim().length > 0);

	// A new result set invalidates whichever row the arrows had landed on.
	$effect(() => {
		void hits;
		activeSuggestion = -1;
	});

	// The field takes the keyboard for itself while it has focus, so the shortcuts
	// that belong to a search are answered here rather than by the window's keymap.
	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			openChosenHit();
			return;
		}
		if (event.key === "s" && (event.ctrlKey || event.metaKey)) {
			event.preventDefault();
			save();
			return;
		}
		moveSuggestion(event);
	}

	function save() {
		if (value.trim() === "") {
			return;
		}
		onSave();
	}

	function openChosenHit() {
		const hit = suggestions[Math.max(0, activeSuggestion)];
		if (!hit) {
			return;
		}
		onOpenHit(hit);
	}

	function moveSuggestion(event: KeyboardEvent) {
		const step = arrowStep(event.key);
		if (step === 0 || suggestions.length === 0) {
			return;
		}
		event.preventDefault();
		activeSuggestion = wrapIndex(activeSuggestion + step, suggestions.length);
	}

	function arrowStep(key: string): number {
		if (key === "ArrowDown") {
			return 1;
		}
		if (key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

	function wrapIndex(index: number, total: number): number {
		return (index + total) % total;
	}

	function hitLocation(hit: proto.Hit): string {
		return parentDirectoryOf(hit.path);
	}

	// The index reports an instant rather than a formatted time; only the preview
	// cache reads it, and it only has to change when the file does.
	function thumbnailSourceFor(hit: proto.Hit): ThumbnailSource {
		return {
			name: hit.name,
			path: hit.path,
			isDir: hit.isDir,
			modTime: String(hit.modTimeUnix),
		};
	}

	function emptyLabel(): string {
		if (isSearching) {
			return "Searching…";
		}
		return "No matches";
	}

	// The dropdown is the whole of the result set the user sees, so it says how
	// much of it is out of sight.
	function footerLabel(): string {
		return [hiddenMatchesLabel(), indexingLabel].filter(Boolean).join(" · ");
	}

	function hiddenMatchesLabel(): string {
		if (hits.length <= suggestions.length) {
			return "";
		}
		if (truncated) {
			return `Showing ${suggestions.length} of the first ${hits.length} matches`;
		}
		return `Showing ${suggestions.length} of ${hits.length} matches`;
	}
</script>

<div class="relative flex min-w-0 flex-1">
	<div
		class="flex h-8 min-w-0 flex-1 items-center gap-2 rounded-md border border-line-faint bg-surface pr-1 pl-2.5"
	>
		<MagnifyingGlassIcon size={14} class="shrink-0 text-faint" />
		{#if scope}
			<span class="shrink-0 rounded bg-raised px-1.5 py-0.5 text-2xs text-muted">{scope}</span>
		{/if}
		<input
			bind:this={input}
			bind:value
			onkeydown={handleKeydown}
			type="text"
			spellcheck="false"
			autocomplete="off"
			{placeholder}
			aria-label={placeholder}
			class="min-w-0 flex-1 bg-transparent text-sm text-default placeholder:text-faint focus:outline-none"
		/>
		{#if isSearching}
			<LoadingSpinner size={14} class="shrink-0 text-faint" />
		{/if}
		<button
			type="button"
			onclick={save}
			disabled={value.trim() === ""}
			aria-label="Save this search"
			title="Save this search"
			class="shrink-0 rounded-md p-1 text-faint transition duration-fast hover:bg-hover hover:text-default disabled:text-line-faint disabled:hover:bg-transparent"
		>
			<BookmarkSimpleIcon size={14} />
		</button>
		<button
			type="button"
			onclick={onClose}
			aria-label="Close search"
			title="Close search"
			class="shrink-0 rounded-md p-1 text-faint transition duration-fast hover:bg-hover hover:text-default"
		>
			<XIcon size={14} />
		</button>
	</div>

	{#if isDropdownOpen}
		<div class="absolute top-full left-0 z-50 mt-2 w-full">
			<Card
				surface="raised"
				padding="none"
				class="flex max-h-96 flex-col gap-1 overflow-y-auto p-2 shadow-lg"
			>
				{#if errorMessage}
					<div class="flex items-center gap-2 p-2">
						<WarningIcon size={16} class="shrink-0 text-red" />
						<div class="min-w-0">
							<p class="text-sm text-default">Search is unavailable</p>
							<p class="truncate text-xs text-dim">{errorMessage}</p>
						</div>
					</div>
				{:else if suggestions.length === 0}
					<p class="p-2 text-sm text-dim">{emptyLabel()}</p>
				{:else}
					{#each suggestions as hit, index (hit.path)}
						<button
							type="button"
							onmousedown={() => onOpenHit(hit)}
							class="flex items-center gap-2 rounded-md p-2 text-left transition duration-fast"
							class:bg-hover={index === activeSuggestion}
							class:hover:bg-hover={index !== activeSuggestion}
						>
							<EntryIcon entry={thumbnailSourceFor(hit)} />
							<span class="min-w-0 flex-1">
								<span class="block truncate text-sm text-default">{hit.name}</span>
								<span class="block truncate text-xs text-dim">{hitLocation(hit)}</span>
							</span>
						</button>
					{/each}
				{/if}

				{#if !errorMessage && footerLabel()}
					<p class="px-2 pt-1 text-2xs text-faint">{footerLabel()}</p>
				{/if}
			</Card>
		</div>
	{/if}
</div>
