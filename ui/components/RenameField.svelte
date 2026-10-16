<!--
	Inline name editor for a row or a grid tile. Enter commits, Escape and losing
	focus cancel, so an accidental click elsewhere never renames.
-->
<script lang="ts">
	import { onMount } from "svelte";
	import { Input, merge, type KeyEvent, type Style } from "@neoworks/svelte-gpui/ui";
	import { colors, radius, space, textSize } from "../theme";

	let {
		initialName,
		onCommit,
		onCancel,
		style = undefined,
	}: {
		initialName: string;
		onCommit: (name: string) => void;
		onCancel: () => void;
		/** Laid over the field's own look, for a tile where it sits under an icon. */
		style?: Style;
	} = $props();

	// svelte-ignore state_referenced_locally
	let value = $state(initialName);
	let input = $state<Input>();

	const look = $derived(
		merge(
			{
				flex: 1,
				minWidth: 0,
				paddingX: space[1],
				paddingY: space[0.5],
				radius: radius.sm,
				background: colors.surface,
				borderWidth: 1,
				borderColor: colors.lineStrong,
				fontSize: textSize.sm,
				color: colors.text,
			},
			style,
		),
	);

	onMount(() => preselectBaseName());

	// The extension stays out of the initial selection so typing replaces only
	// the part being renamed.
	function preselectBaseName() {
		input?.focus();
		input?.select(0, baseNameLength(initialName));
	}

	function baseNameLength(name: string): number {
		const dotIndex = name.lastIndexOf(".");
		if (dotIndex <= 0) {
			return name.length;
		}
		return dotIndex;
	}

	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Enter") {
			event.preventDefault();
			commit();
			return;
		}
		if (event.key === "Escape") {
			event.preventDefault();
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

<Input bind:this={input} bind:value style={look} onkey={handleKey} onblur={onCancel} />
