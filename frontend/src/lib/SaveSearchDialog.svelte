<script lang="ts">
	import { untrack } from "svelte";
	import { Button } from "@neoworks-dev/ui";
	import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
	import Dialog from "./Dialog.svelte";
	import { isNameTaken, type SavedSearch } from "./savedSearches";

	// Names a saved search, both when it is first kept and when it is renamed.
	let {
		title,
		initialName,
		description,
		saved,
		onSave,
		onClose,
	}: {
		title: string;
		initialName: string;
		/** What is being saved, so the name is chosen with the query in view. */
		description: string;
		/** The names already in use, which saving would otherwise replace. */
		saved: SavedSearch[];
		onSave: (name: string) => void;
		onClose: () => void;
	} = $props();

	// The dialog is mounted afresh each time it opens, so the name it starts from
	// is the one it was given and not a value that follows the caller afterwards.
	let name = $state(untrack(() => initialName));
	let field = $state<HTMLInputElement>();

	const replacesAnother = $derived(isNameTaken(saved, name) && name.trim() !== initialName);

	$effect(() => {
		field?.focus();
		field?.select();
	});

	function submit(event: SubmitEvent) {
		event.preventDefault();
		save();
	}

	function save() {
		const trimmed = name.trim();
		if (trimmed === "") {
			return;
		}
		onSave(trimmed);
	}
</script>

<Dialog {title} {onClose}>
	<p class="text-xs text-dim">{description}</p>

	<form onsubmit={submit} class="flex items-center gap-2 rounded-lg bg-raised p-2">
		<MagnifyingGlassIcon size={16} class="shrink-0 text-faint" />
		<input
			bind:this={field}
			bind:value={name}
			type="text"
			placeholder="Name this search"
			spellcheck="false"
			class="min-w-0 flex-1 bg-transparent text-sm text-default placeholder:text-faint focus:outline-none"
		/>
	</form>

	{#if replacesAnother}
		<p class="text-2xs text-faint">A search is already saved under this name and will be replaced.</p>
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onclick={onClose}>Cancel</Button>
		<Button size="sm" variant="surface" onclick={save}>Save search</Button>
	{/snippet}
</Dialog>
