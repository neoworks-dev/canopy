<script lang="ts">
	// Inline name editor for a row or a grid tile. Enter commits, Escape and
	// losing focus cancel, so an accidental click elsewhere never renames.
	let {
		initialName,
		onCommit,
		onCancel,
	}: {
		initialName: string;
		onCommit: (name: string) => void;
		onCancel: () => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	let value = $state(initialName);
	let input = $state<HTMLInputElement>();

	$effect(() => {
		preselectBaseName(input, initialName);
	});

	// The extension stays out of the initial selection so typing replaces only
	// the part being renamed.
	function preselectBaseName(field: HTMLInputElement | undefined, name: string) {
		if (!field) {
			return;
		}
		field.focus();
		field.setSelectionRange(0, baseNameLength(name));
	}

	function baseNameLength(name: string): number {
		const dotIndex = name.lastIndexOf(".");
		if (dotIndex <= 0) {
			return name.length;
		}
		return dotIndex;
	}

	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation();
		if (event.key === "Enter") {
			commit();
			return;
		}
		if (event.key === "Escape") {
			onCancel();
		}
	}

	function commit() {
		const trimmed = value.trim();
		if (trimmed === "" || trimmed === initialName) {
			onCancel();
			return;
		}
		onCommit(trimmed);
	}
</script>

<input
	bind:this={input}
	bind:value
	onkeydown={handleKeydown}
	onblur={onCancel}
	onclick={(event) => event.stopPropagation()}
	ondblclick={(event) => event.stopPropagation()}
	type="text"
	spellcheck="false"
	autocomplete="off"
	aria-label="New name"
	class="min-w-0 flex-1 rounded-md border border-line-strong bg-surface px-1 py-0.5 text-sm text-default focus:outline-none"
/>
