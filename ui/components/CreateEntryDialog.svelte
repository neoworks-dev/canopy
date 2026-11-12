<!-- Names a new file, or a new folder when the name ends in a slash. -->
<script lang="ts">
	import { onMount, tick } from "svelte";
	import { Icon, Text, View } from "@neoworks/svelte-gpui/ui";
	import { filePlus } from "../icons";
	import { colors, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";
	import Dialog from "./Dialog.svelte";
	import Button from "./design/Button.svelte";

	let {
		onCreate,
		onClose,
	}: {
		onCreate: (kind: "file" | "folder", name: string) => void;
		onClose: () => void;
	} = $props();

	let name = $state("");
	let field = $state<BareInput>();

	onMount(async () => {
		await tick();
		field?.focus();
	});

	function create() {
		const trimmed = name.trim();
		if (trimmed === "" || trimmed === "/") {
			return;
		}
		if (trimmed.endsWith("/")) {
			onCreate("folder", trimmed.slice(0, -1));
			return;
		}
		onCreate("file", trimmed);
	}
</script>

<Dialog title="Create" {onClose}>
	<Text style={{ fontSize: textSize.xs, color: colors.dim }}>End the name with / to make a folder.</Text>

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
		<Icon svg={filePlus} size={16} color={colors.faint} style={{ shrink: 0 }} />
		<BareInput bind:this={field} bind:value={name} placeholder="Name" onsubmit={create} />
	</View>

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={onClose}>Cancel</Button>
		<Button size="sm" variant="surface" onpress={create}>Create</Button>
	{/snippet}
</Dialog>
