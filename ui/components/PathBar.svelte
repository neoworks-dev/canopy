<!--
	The location bar. At rest it is the path as breadcrumbs; pressing the empty
	part of it (or the shortcut) turns it into a text field that completes
	folder names, or, after "z ", ranks folders by frecency like the shell's z.
-->
<script lang="ts">
	import { tick } from "svelte";
	import { clipboard } from "@neoworks/svelte-gpui";
	import { Button, Input, Overlay, Text, View, type KeyEvent, type Style } from "@neoworks/svelte-gpui/ui";
	import { check, copy, magnifyingGlass } from "../icons";
	import { ListDirectory, ZoxideQuery } from "../backend/FileService";
	import { defaultSortOrder } from "../lib/sorting";
	import { colors, duration, radius, space, textSize } from "../theme";
	import { floatingPanel } from "./floatingPanel";
	import IconButton from "./IconButton.svelte";
	import PathBreadcrumb from "./PathBreadcrumb.svelte";

	const maxSuggestions = 8;
	const copiedFeedbackMilliseconds = 1200;

	let {
		path,
		isEditing,
		onNavigate,
		onEditingChange,
		onSearch,
	}: {
		path: string;
		isEditing: boolean;
		onNavigate: (path: string) => void;
		onEditingChange: (editing: boolean) => void;
		onSearch: () => void;
	} = $props();

	let draft = $state("");
	let suggestions = $state<string[]>([]);
	let activeSuggestion = $state(-1);
	let input = $state<Input>();
	let isCopied = $state(false);
	let isHovered = $state(false);

	// Guards against a slow directory read landing after the user has typed on.
	let latestCompletion = 0;

	$effect(() => {
		if (!isEditing) {
			return;
		}
		startEditing(path);
	});

	async function startEditing(currentPath: string) {
		draft = currentPath;
		suggestions = [];
		activeSuggestion = -1;
		await tick();
		input?.focus();
		input?.select();
	}

	function stopEditing() {
		onEditingChange(false);
	}

	async function refreshSuggestions(text: string) {
		const request = ++latestCompletion;
		const matches = await suggestionsFor(text);
		if (request !== latestCompletion) {
			return;
		}
		suggestions = matches;
		activeSuggestion = -1;
	}

	async function suggestionsFor(text: string): Promise<string[]> {
		const jumpQuery = zoxideQueryIn(text);
		if (jumpQuery !== null) {
			return frecentMatches(jumpQuery);
		}
		const { directory, prefix } = splitForCompletion(text);
		return directoryMatches(directory, prefix);
	}

	// A "z " prefix turns the bar from completing a path into matching a folder by
	// frecency, the way the shell command of the same name does. An empty query
	// offers the most frecent folders, so the prefix is discoverable.
	function zoxideQueryIn(text: string): string | null {
		if (!text.startsWith("z ")) {
			return null;
		}
		return text.slice(2).trim();
	}

	async function frecentMatches(query: string): Promise<string[]> {
		try {
			const found = await ZoxideQuery(query, path);
			return found.slice(0, maxSuggestions).map((entry) => entry.path);
		} catch {
			return [];
		}
	}

	async function directoryMatches(directory: string, prefix: string): Promise<string[]> {
		try {
			// Completion only reads names, so it takes the default order rather than
			// whatever the listing is currently sorted by.
			const listing = await ListDirectory(directory, defaultSortOrder);
			return listing.entries
				.filter((entry) => entry.isDir && startsWithPrefix(entry.name, prefix))
				.slice(0, maxSuggestions)
				.map((entry) => entry.path);
		} catch {
			return [];
		}
	}

	function startsWithPrefix(name: string, prefix: string): boolean {
		return name.toLowerCase().startsWith(prefix.toLowerCase());
	}

	// The part before the last separator names the directory to read; the rest is
	// the prefix being completed. Text without a separator completes inside the
	// directory currently open.
	function splitForCompletion(text: string): { directory: string; prefix: string } {
		const separatorIndex = text.lastIndexOf("/");
		if (separatorIndex < 0) {
			return { directory: path, prefix: text };
		}
		return {
			directory: text.slice(0, separatorIndex + 1),
			prefix: text.slice(separatorIndex + 1),
		};
	}

	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			stopEditing();
			return;
		}
		if (event.key === "Enter") {
			event.preventDefault();
			submit();
			return;
		}
		if (event.key === "Tab") {
			completeWithSuggestion(event);
			return;
		}
		moveSuggestion(event);
	}

	function submit() {
		const target = chosenSuggestion();
		if (target === "") {
			return;
		}
		onEditingChange(false);
		onNavigate(target);
	}

	function chosenSuggestion(): string {
		if (activeSuggestion >= 0) {
			return suggestions[activeSuggestion];
		}
		// A jump query is not a path, so submitting one takes the best match rather
		// than trying to open the text that was typed.
		if (zoxideQueryIn(draft) !== null) {
			return bestMatch();
		}
		return draft.trim();
	}

	function bestMatch(): string {
		if (suggestions.length === 0) {
			return "";
		}
		return suggestions[0];
	}

	function completeWithSuggestion(event: KeyEvent) {
		event.preventDefault();
		if (suggestions.length === 0) {
			return;
		}
		draft = suggestions[Math.max(0, activeSuggestion)];
		refreshSuggestions(draft);
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

	async function copyPath() {
		await clipboard.write(path);
		isCopied = true;
		setTimeout(() => (isCopied = false), copiedFeedbackMilliseconds);
	}

	function copyIcon(): string {
		if (isCopied) {
			return check;
		}
		return copy;
	}

	function chooseSuggestion(suggestion: string) {
		onEditingChange(false);
		onNavigate(suggestion);
	}

	function suggestionBackground(index: number): string {
		if (index === activeSuggestion) {
			return colors.hover;
		}
		return "transparent";
	}

	function suggestionColor(index: number): string {
		if (index === activeSuggestion) {
			return colors.text;
		}
		return colors.muted;
	}

	const barLook: Style = {
		direction: "row",
		align: "center",
		height: 32,
		minWidth: 0,
		flex: 1,
		gap: space[1],
		paddingLeft: space[1.5],
		paddingRight: space[1],
		radius: radius.md,
		borderWidth: 1,
		borderColor: colors.lineFaint,
		background: colors.surface,
		transition: duration.fast,
	};
</script>

{#if isEditing}
	<View style={{ flex: 1, minWidth: 0 }}>
		<Input
			bind:this={input}
			bind:value={draft}
			oninput={refreshSuggestions}
			onkey={handleKey}
			onblur={stopEditing}
			style={{
				minWidth: 0,
				paddingX: space[2.5],
				paddingY: space[1.5],
				radius: radius.md,
				borderWidth: 1,
				borderColor: colors.lineFaint,
				background: colors.surface,
				fontSize: textSize.sm,
				color: colors.text,
			}}
		/>

		<Overlay side="bottom" align="start" offset={space[2]} matchWidth style={floatingPanel}>
			{#each suggestions as suggestion, index (suggestion)}
				<View
					style={{
						padding: space[2],
						radius: radius.md,
						background: suggestionBackground(index),
						color: suggestionColor(index),
						fontSize: textSize.sm,
					}}
					hover={{ background: colors.hover, color: colors.text }}
					ondown={() => chooseSuggestion(suggestion)}
				>
					<Text style={{ truncate: true }}>{suggestion}</Text>
				</View>
			{/each}
		</Overlay>
	</View>
{:else}
	<View style={barLook} hover={{ borderColor: colors.line }} onhover={(hovered) => (isHovered = hovered)}>
		<PathBreadcrumb {path} {onNavigate} />
		<Button style={{ flex: 1, minWidth: 32, height: "full", cursor: "text" }} onpress={() => onEditingChange(true)} />
		<IconButton icon={copyIcon()} title="Copy path" dimmed={!isHovered && !isCopied} onpress={copyPath} />
		<IconButton icon={magnifyingGlass} title="Search this folder" onpress={onSearch} />
	</View>
{/if}
