<!--
	The search box that replaces the location bar: a field with the folder it is
	confined to, a spinner while it runs, buttons to keep or close the search,
	and a dropdown of the first matches. The index has no text input in the
	design system to build on, so this mirrors the field tokens.
-->
<script lang="ts">
	import { Icon, Overlay, Scroll, Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { bookmarkSimple, magnifyingGlass, warning, x } from "../icons";
	import type { proto } from "../backend/models";
	import { parentDirectoryOf } from "../lib/format";
	import type { ThumbnailSource } from "../lib/thumbnails";
	import { colors, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";
	import EntryIcon from "./EntryIcon.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";
	import IconButton from "./IconButton.svelte";
	import { floatingPanel } from "./floatingPanel";

	// The dropdown is where results live, so it holds enough of them to pick from
	// without covering the listing it is opening things in.
	const maxSuggestions = 10;
	const dropdownMaxHeight = 384;

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
	} = $props();

	let field = $state<BareInput>();
	let activeSuggestion = $state(-1);

	const suggestions = $derived(hits.slice(0, maxSuggestions));
	const isDropdownOpen = $derived(value.trim().length > 0);

	/** Puts the caret in the field, for a shortcut elsewhere. */
	export function focus() {
		field?.focus();
	}

	// A new result set invalidates whichever row the arrows had landed on.
	$effect(() => forgetHighlight(hits));

	function forgetHighlight(_results: proto.Hit[]) {
		activeSuggestion = -1;
	}

	// The field takes the keyboard for itself while it has focus, so the shortcuts
	// that belong to a search are answered here rather than by the window's keymap.
	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			openChosenHit();
			return;
		}
		if (event.key === "s" && (event.ctrl || event.meta)) {
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

	function moveSuggestion(event: KeyEvent) {
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

	function rowBackground(index: number): string {
		if (index === activeSuggestion) {
			return colors.hover;
		}
		return "transparent";
	}
</script>

<View style={{ flex: 1, minWidth: 0 }}>
	<View
		style={{
			direction: "row",
			align: "center",
			gap: space[2],
			height: 32,
			minWidth: 0,
			paddingLeft: space[2.5],
			paddingRight: space[1],
			radius: radius.md,
			borderWidth: 1,
			borderColor: colors.lineFaint,
			background: colors.surface,
		}}
	>
		<Icon svg={magnifyingGlass} size={14} color={colors.faint} style={{ shrink: 0 }} />
		{#if scope}
			<Text
				style={{
					shrink: 0,
					paddingX: space[1.5],
					paddingY: space[0.5],
					radius: radius.xs,
					background: colors.raised,
					fontSize: textSize["2xs"],
					color: colors.muted,
				}}
			>
				{scope}
			</Text>
		{/if}
		<BareInput bind:this={field} bind:value {placeholder} onkey={handleKey} />
		{#if isSearching}
			<LoadingSpinner size={14} color={colors.faint} />
		{/if}
		<IconButton
			icon={bookmarkSimple}
			title="Save this search"
			disabled={value.trim() === ""}
			onpress={save}
		/>
		<IconButton icon={x} title="Close search" onpress={onClose} />
	</View>

	{#if isDropdownOpen}
		<Overlay side="bottom" align="start" offset={space[2]} matchWidth style={{ ...floatingPanel, gap: space[1] }}>
			<Scroll style={{ maxHeight: dropdownMaxHeight }}>
				<View style={{ gap: space[1] }}>
					{#if errorMessage}
						<View style={{ direction: "row", align: "center", gap: space[2], padding: space[2] }}>
							<Icon svg={warning} size={16} color={colors.red} style={{ shrink: 0 }} />
							<View style={{ minWidth: 0 }}>
								<Text style={{ fontSize: textSize.sm, color: colors.text }}>Search is unavailable</Text>
								<Text style={{ truncate: true, fontSize: textSize.xs, color: colors.dim }}>{errorMessage}</Text>
							</View>
						</View>
					{:else if suggestions.length === 0}
						<Text style={{ padding: space[2], fontSize: textSize.sm, color: colors.dim }}>{emptyLabel()}</Text>
					{:else}
						{#each suggestions as hit, index (hit.path)}
							<View
								style={{
									direction: "row",
									align: "center",
									gap: space[2],
									padding: space[2],
									radius: radius.md,
									background: rowBackground(index),
								}}
								hover={{ background: colors.hover }}
								ondown={() => onOpenHit(hit)}
							>
								<EntryIcon entry={thumbnailSourceFor(hit)} />
								<View style={{ flex: 1, minWidth: 0 }}>
									<Text style={{ truncate: true, fontSize: textSize.sm, color: colors.text }}>{hit.name}</Text>
									<Text style={{ truncate: true, fontSize: textSize.xs, color: colors.dim }}>
										{parentDirectoryOf(hit.path)}
									</Text>
								</View>
							</View>
						{/each}
					{/if}
				</View>
			</Scroll>

			{#if !errorMessage && footerLabel()}
				<Text style={{ paddingX: space[2], paddingTop: space[1], fontSize: textSize["2xs"], color: colors.faint }}>
					{footerLabel()}
				</Text>
			{/if}
		</Overlay>
	{/if}
</View>
