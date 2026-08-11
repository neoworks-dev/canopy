<script lang="ts">
	import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
	import { typeAheadMatch } from "./typeAhead";

	// Finding an entry by typing its name used to happen on the listing itself,
	// which meant every letter key belonged to it. Behind a prompt the letters are
	// free to be commands, and the query is visible while it is being typed.
	let {
		names,
		focusIndex,
		onMatch,
		onClose,
	}: {
		names: string[];
		focusIndex: number;
		onMatch: (index: number) => void;
		onClose: (accepted: boolean) => void;
	} = $props();

	let query = $state("");
	let input = $state<HTMLInputElement>();
	let hasMatch = $state(true);

	$effect(() => {
		input?.focus();
	});

	function jumpToMatch() {
		const match = typeAheadMatch(names, query.trim().toLowerCase(), focusIndex);
		hasMatch = match >= 0 || query.trim() === "";
		if (match < 0) {
			return;
		}
		onMatch(match);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			onClose(false);
		}
		if (event.key === "Enter") {
			onClose(true);
		}
	}
</script>

<div
	class="flex shrink-0 items-center gap-2 rounded-md border border-line-faint bg-surface px-2.5 py-1.5"
	class:border-red={!hasMatch}
>
	<MagnifyingGlassIcon size={14} class="shrink-0 text-faint" />
	<input
		bind:this={input}
		bind:value={query}
		oninput={jumpToMatch}
		onkeydown={handleKeydown}
		onblur={() => onClose(true)}
		type="text"
		spellcheck="false"
		autocomplete="off"
		placeholder="Find in this folder"
		aria-label="Find in this folder"
		class="min-w-0 flex-1 bg-transparent text-sm text-default placeholder:text-faint focus:outline-none"
	/>
	<span class="shrink-0 text-2xs text-faint">Esc to cancel</span>
</div>
