<script lang="ts">
	import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
	import XIcon from "phosphor-svelte/lib/XIcon";

	// @neoworks-dev/ui has no text input yet; Select is a dropdown and its
	// internal search box is not exported. This mirrors that box's tokens so the
	// field still looks like the design system.
	let {
		value = $bindable(""),
		placeholder = "Search files",
		input = $bindable<HTMLInputElement | undefined>(),
	}: {
		value?: string;
		placeholder?: string;
		/** Exposed so a shortcut elsewhere can put the caret in this field. */
		input?: HTMLInputElement;
	} = $props();

	function clear() {
		value = "";
		input?.focus();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key !== "Escape") {
			return;
		}
		clear();
	}
</script>

<div
	class="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-line-faint bg-surface px-2.5 py-1.5"
>
	<MagnifyingGlassIcon size={14} class="shrink-0 text-faint" />
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
	{#if value}
		<button
			type="button"
			onclick={clear}
			aria-label="Clear search"
			class="shrink-0 text-faint transition-colors hover:text-default"
		>
			<XIcon size={14} />
		</button>
	{/if}
</div>
