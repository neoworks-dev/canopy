<script lang="ts">
	import type { highlight } from "../../wailsjs/go/models";
	import {
		layoutLines,
		longestLine,
		usedClasses,
		visibleLines,
		type CodeRun,
	} from "./codeCanvas";

	let { tokens }: { tokens: highlight.Token[] } = $props();

	// The canvas covers the viewport rather than the content, so the text is
	// inset by hand where padding would otherwise sit.
	const padding = 8;

	type Metrics = {
		font: string;
		lineHeight: number;
		characterWidth: number;
		defaultColour: string;
	};

	let viewport = $state<HTMLDivElement | null>(null);
	let canvas = $state<HTMLCanvasElement | null>(null);
	let metrics = $state<Metrics | null>(null);
	let scrollTop = $state(0);
	let scrollLeft = $state(0);
	let viewportWidth = $state(0);
	let viewportHeight = $state(0);

	const lines = $derived(layoutLines(tokens));
	const contentWidth = $derived(longestLine(lines) * characterWidth() + padding * 2);
	const contentHeight = $derived(lines.length * lineHeight() + padding * 2);

	// Reading a colour costs a style resolution, so it happens when the file or
	// the theme behind it changes rather than on every frame.
	const colours = $derived(coloursFor(lines, metrics));

	$effect(() => {
		metrics = measure(viewport);
	});

	$effect(() => {
		tokens;
		backToTheTop();
	});

	$effect(() => {
		lines;
		colours;
		scrollTop;
		scrollLeft;
		viewportWidth;
		viewportHeight;
		draw();
	});

	function characterWidth(): number {
		if (!metrics) {
			return 0;
		}
		return metrics.characterWidth;
	}

	function lineHeight(): number {
		if (!metrics) {
			return 0;
		}
		return metrics.lineHeight;
	}

	// The font and the plain text colour come from the element's own styles, so
	// the drawing follows the design system the way the rest of the pane does.
	function measure(element: HTMLDivElement | null): Metrics | null {
		if (!element) {
			return null;
		}
		const style = getComputedStyle(element);
		const font = `${style.fontSize} ${style.fontFamily}`;
		return {
			font,
			lineHeight: lineHeightOf(style),
			characterWidth: characterWidthOf(font),
			defaultColour: style.color,
		};
	}

	function lineHeightOf(style: CSSStyleDeclaration): number {
		const declared = Number.parseFloat(style.lineHeight);
		if (Number.isFinite(declared)) {
			return declared;
		}
		return Number.parseFloat(style.fontSize) * 1.5;
	}

	// One glyph is every glyph in a monospaced face, which is what lets a column
	// be a character offset rather than a measured width.
	function characterWidthOf(font: string): number {
		const context = document.createElement("canvas").getContext("2d");
		if (!context) {
			return 0;
		}
		context.font = font;
		return context.measureText("M").width;
	}

	// A file the pane has just switched to is read from its first line, not from
	// wherever the last one was left.
	function backToTheTop() {
		scrollTop = 0;
		scrollLeft = 0;
		viewport?.scrollTo(0, 0);
	}

	function draw() {
		const context = canvas?.getContext("2d");
		if (!context || !metrics) {
			return;
		}
		prepareCanvas(context, metrics);
		const range = visibleLines(scrollTop, viewportHeight, metrics.lineHeight, lines.length);
		drawRange(context, metrics, range);
	}

	// A canvas is sized in device pixels and drawn in CSS ones, and resizing it
	// clears everything the context was told, the font included.
	function prepareCanvas(context: CanvasRenderingContext2D, drawnWith: Metrics) {
		const ratio = window.devicePixelRatio;
		context.canvas.width = Math.ceil(viewportWidth * ratio);
		context.canvas.height = Math.ceil(viewportHeight * ratio);
		context.scale(ratio, ratio);
		context.font = drawnWith.font;
		context.textBaseline = "top";
	}

	function drawRange(
		context: CanvasRenderingContext2D,
		drawnWith: Metrics,
		range: { first: number; last: number },
	) {
		for (let index = range.first; index < range.last; index++) {
			drawLine(context, drawnWith, index);
		}
	}

	function drawLine(context: CanvasRenderingContext2D, drawnWith: Metrics, index: number) {
		const top = padding + index * drawnWith.lineHeight - scrollTop;
		lines[index].forEach((run) => {
			context.fillStyle = colourOf(run, drawnWith);
			context.fillText(run.text, leftOf(run, drawnWith), top);
		});
	}

	function leftOf(run: CodeRun, drawnWith: Metrics): number {
		return padding + run.column * drawnWith.characterWidth - scrollLeft;
	}

	function colourOf(run: CodeRun, drawnWith: Metrics): string {
		const colour = colours.get(run.className);
		if (!colour) {
			return drawnWith.defaultColour;
		}
		return colour;
	}

	// The stylesheet decides what a token class looks like, so the colours are
	// read back off elements carrying those classes rather than listed again here.
	function coloursFor(drawn: CodeRun[][], drawnWith: Metrics | null): Map<string, string> {
		const found = new Map<string, string>();
		if (!drawnWith) {
			return found;
		}
		usedClasses(drawn).forEach((className) => {
			found.set(className, measuredColour(className, drawnWith));
		});
		return found;
	}

	function measuredColour(className: string, drawnWith: Metrics): string {
		if (!viewport || className === "") {
			return drawnWith.defaultColour;
		}
		const probe = document.createElement("span");
		probe.className = className;
		viewport.append(probe);
		const colour = getComputedStyle(probe).color;
		probe.remove();
		return colour;
	}

	function rememberScroll(event: Event & { currentTarget: HTMLDivElement }) {
		scrollTop = event.currentTarget.scrollTop;
		scrollLeft = event.currentTarget.scrollLeft;
	}
</script>

<div class="relative min-h-0 flex-1 overflow-hidden rounded-lg bg-raised">
	<canvas bind:this={canvas} class="pointer-events-none absolute inset-0 h-full w-full"></canvas>
	<div
		bind:this={viewport}
		bind:clientWidth={viewportWidth}
		bind:clientHeight={viewportHeight}
		onscroll={rememberScroll}
		class="h-full w-full overflow-auto font-mono text-2xs text-default"
	>
		<div style="width: {contentWidth}px; height: {contentHeight}px"></div>
	</div>
</div>
