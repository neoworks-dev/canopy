<script lang="ts">
	import { Card } from "@neoworks-dev/ui";
	import CompassIcon from "phosphor-svelte/lib/CompassIcon";
	import { ZoxideAvailable, ZoxideQuery } from "../../wailsjs/go/main/FileService";
	import type { zoxide } from "../../wailsjs/go/models";

	// The jump prompt: type a fragment of a folder's name and go there, ranked by
	// how often and how recently it was visited. The ranking is zoxide's own, so
	// the same query that works in a shell works here.
	let {
		currentPath,
		onJump,
		onClose,
	}: {
		currentPath: string;
		onJump: (path: string) => void;
		onClose: () => void;
	} = $props();

	let query = $state("");
	let candidates = $state<zoxide.Entry[]>([]);
	let activeIndex = $state(0);
	let isAvailable = $state(true);
	let input = $state<HTMLInputElement>();

	// Guards against a slow lookup landing after the user has typed on.
	let latestLookup = 0;

	$effect(() => {
		input?.focus();
		checkAvailability();
		refresh("");
	});

	async function checkAvailability() {
		try {
			isAvailable = await ZoxideAvailable();
		} catch {
			isAvailable = false;
		}
	}

	async function refresh(text: string) {
		const lookup = ++latestLookup;
		const found = await lookUp(text);
		if (lookup !== latestLookup) {
			return;
		}
		candidates = found;
		activeIndex = 0;
	}

	async function lookUp(text: string): Promise<zoxide.Entry[]> {
		try {
			return await ZoxideQuery(text, currentPath);
		} catch {
			return [];
		}
	}

	function handleInput() {
		refresh(query);
	}

	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			jumpToActive();
			return;
		}
		moveActive(event);
	}

	function jumpToActive() {
		const target = candidates[activeIndex];
		if (!target) {
			return;
		}
		onJump(target.path);
	}

	function moveActive(event: KeyboardEvent) {
		const step = arrowStep(event.key);
		if (step === 0 || candidates.length === 0) {
			return;
		}
		event.preventDefault();
		activeIndex = (activeIndex + step + candidates.length) % candidates.length;
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

	function emptyMessage(): string {
		if (!isAvailable) {
			return "zoxide is not installed";
		}
		return "No folder matches";
	}
</script>

<div class="fixed inset-x-0 top-16 z-50 mx-auto w-full max-w-xl px-4">
	<Card surface="raised" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
		<div class="flex items-center gap-2 rounded-md border border-line-faint bg-surface px-2.5 py-1.5">
			<CompassIcon size={14} class="shrink-0 text-faint" />
			<input
				bind:this={input}
				bind:value={query}
				oninput={handleInput}
				onkeydown={handleKeydown}
				onblur={onClose}
				type="text"
				spellcheck="false"
				autocomplete="off"
				placeholder="Jump to a folder"
				aria-label="Jump to a folder"
				class="min-w-0 flex-1 bg-transparent text-sm text-default placeholder:text-faint focus:outline-none"
			/>
		</div>

		{#if candidates.length === 0}
			<p class="p-2 text-xs text-dim">{emptyMessage()}</p>
		{:else}
			{#each candidates as candidate, index (candidate.path)}
				<button
					type="button"
					onmousedown={() => onJump(candidate.path)}
					onmouseenter={() => (activeIndex = index)}
					class="flex items-baseline gap-2 rounded-md p-2 text-left text-sm transition duration-fast"
					class:bg-hover={index === activeIndex}
					class:text-default={index === activeIndex}
					class:text-muted={index !== activeIndex}
				>
					<span class="min-w-0 flex-1 truncate">{candidate.path}</span>
					<span class="shrink-0 text-2xs text-faint">{candidate.score.toFixed(0)}</span>
				</button>
			{/each}
		{/if}
	</Card>
</div>
