<!--
	The jump prompt: type a fragment of a folder's name and go there, ranked by
	how often and how recently it was visited. The ranking is zoxide's own, so
	the same query that works in a shell works here.
-->
<script lang="ts">
	import { onMount } from "svelte";
	import { Text, View, type KeyEvent } from "@neoworks/svelte-gpui/ui";
	import { compass } from "../icons";
	import { ZoxideAvailable, ZoxideQuery } from "../backend/FileService";
	import type { zoxide } from "../backend/models";
	import { colors, duration, radius, space, textSize } from "../theme";
	import PromptPanel from "./PromptPanel.svelte";

	let {
		currentPath,
		onJump,
		onClose,
	}: {
		currentPath: string;
		onJump: (path: string) => void;
		onClose: () => void;
	} = $props();

	let query = $state("");
	let candidates = $state<zoxide.Entry[]>([]);
	let activeIndex = $state(0);
	let isAvailable = $state(true);

	// Guards against a slow lookup landing after the user has typed on.
	let latestLookup = 0;

	onMount(() => {
		checkAvailability();
		refresh("");
	});

	async function checkAvailability() {
		try {
			isAvailable = await ZoxideAvailable();
		} catch {
			isAvailable = false;
		}
	}

	async function refresh(text: string) {
		const lookup = ++latestLookup;
		const found = await lookUp(text);
		if (lookup !== latestLookup) {
			return;
		}
		candidates = found;
		activeIndex = 0;
	}

	async function lookUp(text: string): Promise<zoxide.Entry[]> {
		try {
			return await ZoxideQuery(text, currentPath);
		} catch {
			return [];
		}
	}

	function handleKey(event: KeyEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			jumpToActive();
			return;
		}
		moveActive(event);
	}

	function jumpToActive() {
		const target = candidates[activeIndex];
		if (!target) {
			return;
		}
		onJump(target.path);
	}

	function moveActive(event: KeyEvent) {
		const step = arrowStep(event.key);
		if (step === 0 || candidates.length === 0) {
			return;
		}
		event.preventDefault();
		activeIndex = (activeIndex + step + candidates.length) % candidates.length;
	}

	function arrowStep(key: string): number {
		if (key === "ArrowDown") {
			return 1;
		}
		if (key === "ArrowUp") {
			return -1;
		}
		return 0;
	}

	function emptyMessage(): string {
		if (!isAvailable) {
			return "zoxide is not installed";
		}
		return "No folder matches";
	}

	function highlightOnHover(hovered: boolean, index: number) {
		if (hovered) {
			activeIndex = index;
		}
	}

	function rowBackground(index: number): string {
		if (index === activeIndex) {
			return colors.hover;
		}
		return "transparent";
	}

	function rowColor(index: number): string {
		if (index === activeIndex) {
			return colors.text;
		}
		return colors.muted;
	}
</script>

<PromptPanel
	icon={compass}
	placeholder="Jump to a folder"
	bind:value={query}
	oninput={refresh}
	onkey={handleKey}
	{onClose}
>
	{#if candidates.length === 0}
		<Text style={{ padding: space[2], fontSize: textSize.xs, color: colors.dim }}>{emptyMessage()}</Text>
	{:else}
		{#each candidates as candidate, index (candidate.path)}
			<View
				style={{
					direction: "row",
					align: "center",
					gap: space[2],
					padding: space[2],
					radius: radius.md,
					fontSize: textSize.sm,
					background: rowBackground(index),
					color: rowColor(index),
					transition: duration.fast,
				}}
				onhover={(hovered) => highlightOnHover(hovered, index)}
				onpress={() => onJump(candidate.path)}
			>
				<Text style={{ flex: 1, minWidth: 0, truncate: true }}>{candidate.path}</Text>
				<Text style={{ shrink: 0, fontSize: textSize["2xs"], color: colors.faint }}>
					{candidate.score.toFixed(0)}
				</Text>
			</View>
		{/each}
	{/if}
</PromptPanel>
