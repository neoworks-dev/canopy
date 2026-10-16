<!--
	Tags the selection: the ones a single file already carries (each removable),
	a field for a new one, and the known names to pick from so tags stay
	consistent.
-->
<script lang="ts">
	import { onMount, tick } from "svelte";
	import { Button as PlainButton, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
	import { tag as tagIcon, x } from "../icons";
	import { colors, duration, radius, space, textSize } from "../theme";
	import BareInput from "./BareInput.svelte";
	import Dialog from "./Dialog.svelte";
	import IconButton from "./IconButton.svelte";
	import Button from "./design/Button.svelte";

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
	let field = $state<BareInput>();

	const suggestions = $derived(known.filter((candidate) => !applied.includes(candidate)));

	// Dialog moves focus to its panel once it has mounted, so the field asks after.
	onMount(async () => {
		await tick();
		field?.focus();
	});

	function apply(candidate: string) {
		const trimmed = candidate.trim();
		if (!trimmed) {
			return;
		}
		name = "";
		onApply(trimmed);
	}
</script>

<Dialog {title} {onClose}>
	{#if applied.length > 0}
		<View style={{ direction: "row", wrap: true, gap: space[2] }}>
			{#each applied as tag (tag)}
				<View
					style={{
						direction: "row",
						align: "center",
						gap: space[1],
						paddingY: space[1],
						paddingRight: space[1],
						paddingLeft: space[2],
						radius: radius.md,
						background: colors.raised,
						fontSize: textSize.xs,
						color: colors.muted,
					}}
				>
					<Text>{tag}</Text>
					<IconButton icon={x} size={10} title="Remove the tag {tag}" onpress={() => onRemove(tag)} />
				</View>
			{/each}
		</View>
	{/if}

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
		<Icon svg={tagIcon} size={16} color={colors.faint} style={{ shrink: 0 }} />
		<BareInput bind:this={field} bind:value={name} placeholder="New tag" onsubmit={apply} />
	</View>

	{#if suggestions.length > 0}
		<View style={{ direction: "row", wrap: true, gap: space[2] }}>
			{#each suggestions as suggestion (suggestion)}
				<PlainButton
					style={{
						paddingX: space[2],
						paddingY: space[1],
						radius: radius.md,
						background: colors.raised,
						fontSize: textSize.xs,
						color: colors.muted,
						transition: duration.fast,
					}}
					hover={{ background: colors.hover, color: colors.text }}
					onpress={() => apply(suggestion)}
				>
					{suggestion}
				</PlainButton>
			{/each}
		</View>
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={onClose}>Done</Button>
		<Button size="sm" variant="surface" onpress={() => apply(name)}>Add tag</Button>
	{/snippet}
</Dialog>
