<!-- Names a saved search, both when it is first kept and when it is renamed. -->
<script lang="ts">
	import { onMount, tick, untrack } from "svelte";
	import { Icon, Text, View } from "@neoworks/svelte-gpui/ui";
	import { magnifyingGlass } from "../icons";
	import { isNameTaken, type SavedSearch } from "../lib/savedSearches";
	import { colors, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";
	import Dialog from "./Dialog.svelte";
	import Button from "./design/Button.svelte";

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
	let field = $state<BareInput>();

	const replacesAnother = $derived(isNameTaken(saved, name) && name.trim() !== initialName);

	// Dialog moves focus to its panel once it has mounted, so the field asks after.
	onMount(async () => {
		await tick();
		field?.focus();
		field?.select();
	});

	function save() {
		const trimmed = name.trim();
		if (trimmed === "") {
			return;
		}
		onSave(trimmed);
	}
</script>

<Dialog {title} {onClose}>
	<Text style={{ fontSize: textSize.xs, color: colors.dim }}>{description}</Text>

	<View
		style={{
			direction: "row",
			align: "center",
			gap: space[2],
			padding: space[2],
			radius: radius.lg,
			background: colors.raised,
		}}
	>
		<Icon svg={magnifyingGlass} size={16} color={colors.faint} style={{ shrink: 0 }} />
		<BareInput bind:this={field} bind:value={name} placeholder="Name this search" onsubmit={save} />
	</View>

	{#if replacesAnother}
		<Text style={{ fontSize: textSize["2xs"], color: colors.faint }}>
			A search is already saved under this name and will be replaced.
		</Text>
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={onClose}>Cancel</Button>
		<Button size="sm" variant="surface" onpress={save}>Save search</Button>
	{/snippet}
</Dialog>
