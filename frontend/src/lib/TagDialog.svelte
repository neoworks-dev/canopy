<script lang="ts">
	import { Button } from "@neoworks-dev/ui";
	import TagIcon from "phosphor-svelte/lib/TagIcon";
	import XIcon from "phosphor-svelte/lib/XIcon";
	import Dialog from "./Dialog.svelte";

	let {
		title,
		known,
		applied,
		onApply,
		onRemove,
		onClose,
	}: {
		title: string;
		/** Names already in use, offered so tags stay consistent. */
		known: string[];
		/** What the selection carries now, when it is a single file. */
		applied: string[];
		onApply: (name: string) => void;
		onRemove: (name: string) => void;
		onClose: () => void;
	} = $props();

	let name = $state("");
	let field = $state<HTMLInputElement>();

	const suggestions = $derived(known.filter((candidate) => !applied.includes(candidate)));

	$effect(() => {
		field?.focus();
	});

	function submit(event: SubmitEvent) {
		event.preventDefault();
		apply(name);
	}

	function apply(candidate: string) {
		const trimmed = candidate.trim();
		if (!trimmed) {
			return;
		}
		name = "";
		onApply(trimmed);
	}
</script>

<Dialog {title} onClose={onClose}>
	{#if applied.length > 0}
		<div class="flex flex-wrap gap-2">
			{#each applied as tag (tag)}
				<span class="flex items-center gap-1 rounded-md bg-raised py-1 pr-1 pl-2 text-xs text-muted">
					{tag}
					<button
						type="button"
						onclick={() => onRemove(tag)}
						aria-label="Remove the tag {tag}"
						class="rounded-sm p-1 text-faint transition duration-fast hover:bg-hover hover:text-default"
					>
						<XIcon size={10} />
					</button>
				</span>
			{/each}
		</div>
	{/if}

	<form onsubmit={submit} class="flex items-center gap-2 rounded-lg bg-raised p-2">
		<TagIcon size={16} class="shrink-0 text-faint" />
		<input
			bind:this={field}
			bind:value={name}
			type="text"
			placeholder="New tag"
			spellcheck="false"
			class="min-w-0 flex-1 bg-transparent text-sm text-default placeholder:text-faint focus:outline-none"
		/>
	</form>

	{#if suggestions.length > 0}
		<div class="flex flex-wrap gap-2">
			{#each suggestions as suggestion (suggestion)}
				<button
					type="button"
					onclick={() => apply(suggestion)}
					class="rounded-md bg-raised px-2 py-1 text-xs text-muted transition duration-fast hover:bg-hover hover:text-default"
				>
					{suggestion}
				</button>
			{/each}
		</div>
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onclick={onClose}>Done</Button>
		<Button size="sm" variant="surface" onclick={() => apply(name)}>Add tag</Button>
	{/snippet}
</Dialog>
