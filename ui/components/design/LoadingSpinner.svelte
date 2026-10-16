<!--
	The design system's spinner: a faint ring and a bright quarter arc going round.
	Styles cannot rotate, so the ring is redrawn on a canvas each frame.
-->
<script lang="ts">
	import { Canvas } from "@neoworks/svelte-gpui/ui";
	import { colors } from "../../theme";

	let {
		size = 24,
		thickness = 2.5,
		color = colors.muted,
	}: {
		/** Diameter in pixels. */
		size?: number;
		/** Stroke width, in a 24-pixel box's units as the web spinner counts it. */
		thickness?: number;
		color?: string;
	} = $props();

	const segments = 32;
	const turnMilliseconds = 1000;
	const trackOpacityHex = "33";

	let canvas: Canvas;

	$effect(() => {
		let frame = requestAnimationFrame(draw);
		function draw(now: number) {
			paint(now);
			frame = requestAnimationFrame(draw);
		}
		return () => cancelAnimationFrame(frame);
	});

	function paint(now: number) {
		if (!canvas) {
			return;
		}
		const scale = size / 24;
		const pen = { width: thickness * scale };
		const center = size / 2;
		const ringRadius = 9 * scale;
		const start = ((now % turnMilliseconds) / turnMilliseconds) * Math.PI * 2;
		canvas.clear();
		strokeArc(center, ringRadius, 0, Math.PI * 2, { ...pen, color: `${color}${trackOpacityHex}` });
		strokeArc(center, ringRadius, start, start + Math.PI / 2, { ...pen, color });
	}

	function strokeArc(center: number, ringRadius: number, from: number, to: number, pen: { width: number; color: string }) {
		const steps = Math.max(2, Math.round((segments * (to - from)) / (Math.PI * 2)));
		canvas.begin(center + Math.cos(from) * ringRadius, center + Math.sin(from) * ringRadius, pen);
		for (let step = 1; step <= steps; step++) {
			const angle = from + ((to - from) * step) / steps;
			canvas.lineTo(center + Math.cos(angle) * ringRadius, center + Math.sin(angle) * ringRadius);
		}
		canvas.end();
	}
</script>

<Canvas bind:this={canvas} style={{ size, shrink: 0 }} />
