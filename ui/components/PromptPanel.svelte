<!--
	The shell of the quick prompts (jump to a folder, run a command): a field
	with an icon, and below it whatever the prompt lists. It sits near the top of
	the window over a transparent backdrop, and a press outside it closes it.
-->
<script lang="ts">
	import { onMount, type Snippet } from "svelte";
	import { Icon, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { colors, radius, space } from "../theme";
	import BareInput from "./BareInput.svelte";
	import { floatingPanel } from "./floatingPanel";

	const topOffset = 64;
	const maxWidth = 576;

	let {
		icon,
		placeholder,
		value = $bindable(""),
		oninput = undefined,
		onkey,
		onClose,
		children,
	}: {
		/** SVG markup. */
		icon: string;
		placeholder: string;
		value?: string;
		oninput?: (value: string) => void;
		onkey: (event: KeyEvent) => void;
		onClose: () => void;
		children: Snippet;
	} = $props();

	let field = $state<BareInput>();

	onMount(() => field?.focus());
</script>

<!-- The inner press handler gives the panel a hitbox, so a press on it is not a press on the backdrop. -->
<View
	style={{ position: "absolute", inset: 0, align: "center", paddingTop: topOffset, paddingX: space[4] }}
	onpress={onClose}
>
	<View style={{ width: "full", maxWidth }} onpress={() => undefined}>
		<View style={floatingPanel}>
			<View
				style={{
					direction: "row",
					align: "center",
					gap: space[2],
					paddingX: space[2.5],
					paddingY: space[1.5],
					radius: radius.md,
					borderWidth: 1,
					borderColor: colors.lineFaint,
					background: colors.surface,
				}}
			>
				<Icon svg={icon} size={14} color={colors.faint} style={{ shrink: 0 }} />
				<BareInput bind:this={field} bind:value {placeholder} {oninput} {onkey} />
			</View>
			{@render children()}
		</View>
	</View>
</View>
