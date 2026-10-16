<!--
	The handle in the gap between two floating panes rather than on either
	pane's border, so the highlight never sits on a card edge. Drag it, or press
	the arrow keys while it has focus.
-->
<script lang="ts">
	import { Tooltip, View, type KeyEvent, type PointerEvent } from "@neoworks/svelte-gpui/ui";
	import { paneGutterWidth, paneResizeStep, paneWidthAfterDrag, type PaneSide } from "../lib/panes";
	import { colors, duration, radius, space } from "../theme";

	let {
		side,
		width,
		label,
		onWidth,
	}: {
		side: PaneSide;
		width: number;
		/** What the handle resizes, shown as a tooltip. */
		label: string;
		onWidth: (width: number) => void;
	} = $props();

	let isDragging = $state(false);
	let isHovered = $state(false);
	let isFocused = $state(false);

	// The handle moves as the pane changes width, so the drag is measured in the
	// window rather than from the handle itself.
	let startX = 0;
	let startWidth = 0;

	function startResize(event: PointerEvent) {
		if (event.button !== 0) {
			return;
		}
		isDragging = true;
		startX = event.windowX;
		startWidth = width;
	}

	function resize(event: PointerEvent) {
		if (!isDragging) {
			return;
		}
		onWidth(paneWidthAfterDrag(side, startWidth, event.windowX - startX));
	}

	function stopResize() {
		isDragging = false;
	}

	function handleKey(event: KeyEvent) {
		const step = arrowStep(event.key);
		if (step === 0) {
			return;
		}
		event.preventDefault();
		onWidth(paneWidthAfterDrag(side, width, step));
	}

	function arrowStep(key: string): number {
		if (key === "ArrowRight") {
			return paneResizeStep;
		}
		if (key === "ArrowLeft") {
			return -paneResizeStep;
		}
		return 0;
	}

	function barColor(): string {
		if (isDragging) {
			return colors.action;
		}
		if (isHovered || isFocused) {
			return colors.lineStrong;
		}
		return "transparent";
	}
</script>

<Tooltip
	text={label}
	side="top"
	delay={900}
	disabled={isDragging}
	anchorStyle={{ width: paneGutterWidth, shrink: 0, alignSelf: "stretch" }}
>
	<View
		focusable
		style={{ flex: 1, align: "center", cursor: "col-resize" }}
		ondown={startResize}
		onmove={resize}
		onup={stopResize}
		onhover={(hovered) => (isHovered = hovered)}
		onkey={handleKey}
		onfocus={() => (isFocused = true)}
		onblur={() => (isFocused = false)}
	>
		<View
			style={{
				flex: 1,
				width: 2,
				marginY: space[1.5],
				radius: radius.full,
				background: barColor(),
				transition: duration.fast,
			}}
		/>
	</View>
</Tooltip>
