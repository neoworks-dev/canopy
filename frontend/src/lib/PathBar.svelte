<script lang="ts">
	import { Card } from "@neoworks-dev/ui";
	import { ListDirectory, ZoxideQuery } from "../../wailsjs/go/main/FileService";
	import PathBreadcrumb from "./PathBreadcrumb.svelte";
	import { defaultSortOrder } from "./sorting";

	const maxSuggestions = 8;

	let {
		path,
		isEditing,
		onNavigate,
		onEditingChange,
	}: {
		path: string;
		isEditing: boolean;
		onNavigate: (path: string) => void;
		onEditingChange: (editing: boolean) => void;
	} = $props();

	let draft = $state("");
	let suggestions = $state<string[]>([]);
	let activeSuggestion = $state(-1);
	let input = $state<HTMLInputElement>();

	// Guards against a slow directory read landing after the user has typed on.
	let latestCompletion = 0;

	$effect(() => {
		if (!isEditing) {
			return;
		}
		startEditing(path);
	});

	function startEditing(currentPath: string) {
		draft = currentPath;
		suggestions = [];
		activeSuggestion = -1;
		input?.focus();
		input?.select();
	}

	function stopEditing() {
		onEditingChange(false);
	}

	function handleInput() {
		refreshSuggestions(draft);
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

	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			stopEditing();
			return;
		}
		if (event.key === "Enter") {
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

	function completeWithSuggestion(event: KeyboardEvent) {
		if (suggestions.length === 0) {
			return;
		}
		event.preventDefault();
		draft = suggestions[Math.max(0, activeSuggestion)];
		refreshSuggestions(draft);
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

	function chooseSuggestion(suggestion: string) {
		onEditingChange(false);
		onNavigate(suggestion);
	}
</script>

{#if isEditing}
	<div class="relative flex min-w-0 flex-1">
		<input
			bind:this={input}
			bind:value={draft}
			oninput={handleInput}
			onkeydown={handleKeydown}
			onblur={stopEditing}
			type="text"
			spellcheck="false"
			autocomplete="off"
			aria-label="Location"
			class="min-w-0 flex-1 rounded-md border border-line-faint bg-surface px-2.5 py-1.5 text-sm text-default focus:outline-none"
		/>

		{#if suggestions.length > 0}
			<div class="absolute top-full left-0 z-50 mt-2 w-full">
				<Card surface="raised" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
					{#each suggestions as suggestion, index (suggestion)}
						<button
							type="button"
							onmousedown={() => chooseSuggestion(suggestion)}
							class="truncate rounded-md p-2 text-left text-sm transition duration-fast"
							class:bg-hover={index === activeSuggestion}
							class:text-default={index === activeSuggestion}
							class:text-muted={index !== activeSuggestion}
							class:hover:bg-hover={index !== activeSuggestion}
							class:hover:text-default={index !== activeSuggestion}
						>
							{suggestion}
						</button>
					{/each}
				</Card>
			</div>
		{/if}
	</div>
{:else}
	<PathBreadcrumb {path} {onNavigate} />
{/if}
