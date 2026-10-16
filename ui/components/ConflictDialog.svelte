<!-- Asks what to do with an incoming item whose destination already exists. -->
<script lang="ts">
	import { Text, View } from "@neoworks/svelte-gpui/ui";
	import type { ConflictPrompt, ConflictResolution } from "../lib/fileops";
	import { colors, radius, space, textSize } from "../theme";
	import CheckRow from "./CheckRow.svelte";
	import Dialog from "./Dialog.svelte";
	import Button from "./design/Button.svelte";

	let {
		prompt,
		onResolve,
	}: {
		prompt: ConflictPrompt;
		onResolve: (resolution: ConflictResolution, applyToAll: boolean) => void;
	} = $props();

	let applyToAll = $state(false);

	const name = $derived(nameOf(prompt.destinationPath));

	function resolve(resolution: ConflictResolution) {
		onResolve(resolution, applyToAll);
	}

	function cancel() {
		onResolve("cancel", false);
	}

	function nameOf(path: string): string {
		const parts = path.split("/");
		return parts[parts.length - 1];
	}
</script>

{#snippet pathBlock(caption: string, path: string)}
	<View style={{ minWidth: 0 }}>
		<Text style={{ fontSize: textSize["2xs"], uppercase: true, color: colors.faint }}>{caption}</Text>
		<Text style={{ truncate: true, fontSize: textSize.xs, color: colors.muted }}>{path}</Text>
	</View>
{/snippet}

<Dialog title="{name} already exists" onClose={cancel}>
	<Text style={{ fontSize: textSize.sm, color: colors.muted }}>
		Something is already at the destination. Choose what happens to the incoming item.
	</Text>
	<View style={{ gap: space[2], padding: space[2], radius: radius.lg, background: colors.raised }}>
		{@render pathBlock("Incoming", prompt.sourcePath)}
		{@render pathBlock("Existing", prompt.destinationPath)}
	</View>
	<CheckRow bind:checked={applyToAll} label="Apply to all remaining conflicts" />

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={cancel}>Cancel</Button>
		<Button size="sm" variant="ghost" onpress={() => resolve("skip")}>Skip</Button>
		<Button size="sm" variant="surface" onpress={() => resolve("keepBoth")}>Keep both</Button>
		<Button size="sm" variant="surface" onpress={() => resolve("replace")}>Replace</Button>
	{/snippet}
</Dialog>
