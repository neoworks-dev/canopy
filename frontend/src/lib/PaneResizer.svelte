<script lang="ts">
	// The handle lives in the gap between two floating panes rather than on either
	// pane's border, so the highlight never sits on a card edge. A negative inset
	// widens the pointer target past the visible gap without shifting the layout.
	import {
		maximumWidthFor,
		minimumPaneWidth,
		paneResizeStep,
		paneWidthAfterDrag,
		type PaneSide,
	} from "./panes";

	let {
		side,
		width,
		label,
		onWidth,
	}: {
		side: PaneSide;
		width: number;
		label: string;
		onWidth: (width: number) => void;
	} = $props();

	let isDragging = $state(false);

	function startResize(event: PointerEvent) {
		event.preventDefault();
		isDragging = true;
		const startX = event.clientX;
		const startWidth = width;
		const move = (moved: PointerEvent) => {
			onWidth(paneWidthAfterDrag(side, startWidth, moved.clientX - startX));
		};
		const stop = () => {
			isDragging = false;
			window.removeEventListener("pointermove", move);
			window.removeEventListener("pointerup", stop);
		};
		window.addEventListener("pointermove", move);
		window.addEventListener("pointerup", stop);
	}

	function handleKeydown(event: KeyboardEvent) {
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
</script>

<!-- A focusable separator is the ARIA window-splitter pattern: it carries the
     width it controls and answers to the arrow keys, which is what the two
     warnings below are about. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	role="separator"
	aria-orientation="vertical"
	aria-label={label}
	aria-valuenow={width}
	aria-valuemin={minimumPaneWidth}
	aria-valuemax={maximumWidthFor(side)}
	tabindex="0"
	onpointerdown={startResize}
	onkeydown={handleKeydown}
	class="group relative flex w-2 shrink-0 cursor-col-resize items-stretch justify-center focus-visible:outline-none"
>
	<div class="absolute inset-y-0 -right-1 -left-1"></div>
	<div
		class="my-1.5 w-0.5 rounded-full transition-colors duration-fast"
		class:bg-action={isDragging}
		class:group-hover:bg-line-strong={!isDragging}
		class:group-focus-visible:bg-line-strong={!isDragging}
	></div>
</div>
