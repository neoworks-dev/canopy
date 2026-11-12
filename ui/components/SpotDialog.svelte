<!--
	Yazi's spot: everything about the hovered entry in one table. h and l swipe
	to the previous and next entry, j and k walk the lines, c c copies the
	highlighted value, Tab or Escape closes.
-->
<script lang="ts">
	import { clipboard } from "@neoworks/svelte-gpui";
	import { Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import type { filesystem } from "../backend/models";
	import { spotLines } from "../lib/spot";
	import { colors, radius, space, textSize } from "../theme";
	import Dialog from "./Dialog.svelte";

	const labelWidth = 96;

	let {
		entry,
		onSwipe,
		onClose,
	}: {
		entry: filesystem.Entry;
		/** Moves the listing's cursor by one, so the spot follows it. */
		onSwipe: (step: number) => void;
		onClose: () => void;
	} = $props();

	let lineIndex = $state(0);
	// The first c of "c c", waiting for the second.
	let isCopyPending = $state(false);

	const lines = $derived(spotLines(entry));

	function handleKey(event: KeyEvent) {
		if (event.key === "Tab") {
			event.preventDefault();
			onClose();
			return;
		}
		if (event.key === "c") {
			copyOnSecondPress();
			return;
		}
		isCopyPending = false;
		moveFor(event.key);
	}

	function copyOnSecondPress() {
		if (!isCopyPending) {
			isCopyPending = true;
			return;
		}
		isCopyPending = false;
		clipboard.write(lines[lineIndex].value);
	}

	function moveFor(key: string) {
		if (key === "h" || key === "ArrowLeft") {
			onSwipe(-1);
			return;
		}
		if (key === "l" || key === "ArrowRight") {
			onSwipe(1);
			return;
		}
		moveLine(lineStep(key));
	}

	function lineStep(key: string): number {
		if (key === "j" || key === "ArrowDown") {
			return 1;
		}
		if (key === "k" || key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

	function moveLine(step: number) {
		lineIndex = Math.max(0, Math.min(lines.length - 1, lineIndex + step));
	}

	function lineBackground(index: number): string {
		if (index === lineIndex) {
			return colors.hover;
		}
		return "transparent";
	}
</script>

<Dialog title={entry.name} {onClose} onKey={handleKey}>
	<View style={{ gap: 1 }}>
		{#each lines as line, index (line.label)}
			<View
				style={{
					direction: "row",
					align: "baseline",
					gap: space[2],
					paddingX: space[2],
					paddingY: space[1],
					radius: radius.sm,
					background: lineBackground(index),
				}}
			>
				<Text style={{ width: labelWidth, shrink: 0, fontSize: textSize["2xs"], uppercase: true, color: colors.faint }}>
					{line.label}
				</Text>
				<Text style={{ flex: 1, minWidth: 0, fontSize: textSize.xs, color: colors.muted }}>{line.value}</Text>
			</View>
		{/each}
	</View>

	<Text style={{ fontSize: textSize["2xs"], color: colors.faint }}>h l swipe · j k move · c c copy · Tab close</Text>
</Dialog>
