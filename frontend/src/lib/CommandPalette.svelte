<script lang="ts">
	import { Card } from "@neoworks-dev/ui";
	import MagnifyingGlassIcon from "phosphor-svelte/lib/MagnifyingGlassIcon";
	import type { Command } from "./commands";
	import { availableCommands, rankCommands, titleSegments } from "./palette";
	import type { WhenContext } from "./whenClause";

	// Every command the explorer can run right now, searchable by name. The
	// registry is the same one the keymap and the context menu read, so a command
	// reaches all three by being registered once.
	//
	// The context is the one captured when the palette opened, because the listing
	// loses focus to the input here and a command has to run against the state the
	// user was looking at when they reached for it.
	let {
		commands,
		context,
		shortcutOf,
		onRun,
		onClose,
	}: {
		commands: Command[];
		context: WhenContext;
		shortcutOf: (id: string) => string;
		onRun: (id: string) => void;
		onClose: () => void;
	} = $props();

	let query = $state("");
	let activeIndex = $state(0);
	let input = $state<HTMLInputElement>();
	let rows = $state<HTMLButtonElement[]>([]);

	const matches = $derived(rankCommands(availableCommands(commands, context), query));

	$effect(() => {
		input?.focus();
	});

	// The list scrolls, so arrowing past its edge has to bring the row along.
	$effect(() => {
		rows[activeIndex]?.scrollIntoView({ block: "nearest" });
	});

	function handleInput() {
		activeIndex = 0;
	}

	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation();
		if (event.key === "Escape") {
			onClose();
			return;
		}
		if (event.key === "Enter") {
			runActive();
			return;
		}
		moveActive(event);
	}

	function runActive() {
		const match = matches[activeIndex];
		if (!match) {
			return;
		}
		onRun(match.command.id);
	}

	function moveActive(event: KeyboardEvent) {
		const step = arrowStep(event.key);
		if (step === 0 || matches.length === 0) {
			return;
		}
		event.preventDefault();
		activeIndex = (activeIndex + step + matches.length) % matches.length;
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
</script>

<div class="fixed inset-x-0 top-16 z-50 mx-auto w-full max-w-xl px-4">
	<Card surface="raised" padding="none" class="flex flex-col gap-2 p-2 shadow-lg">
		<div class="flex items-center gap-2 rounded-md border border-line-faint bg-surface px-2.5 py-1.5">
			<MagnifyingGlassIcon size={14} class="shrink-0 text-faint" />
			<input
				bind:this={input}
				bind:value={query}
				oninput={handleInput}
				onkeydown={handleKeydown}
				onblur={onClose}
				type="text"
				spellcheck="false"
				autocomplete="off"
				placeholder="Run a command"
				aria-label="Run a command"
				class="min-w-0 flex-1 bg-transparent text-sm text-default placeholder:text-faint focus:outline-none"
			/>
		</div>

		{#if matches.length === 0}
			<p class="p-2 text-xs text-dim">No command matches</p>
		{:else}
			<div class="flex max-h-96 flex-col gap-0.5 overflow-y-auto">
				{#each matches as match, index (match.command.id)}
					{@const IconComponent = match.command.icon}
					<button
						bind:this={rows[index]}
						type="button"
						onmousedown={() => onRun(match.command.id)}
						onmouseenter={() => (activeIndex = index)}
						class="flex shrink-0 items-center gap-2 rounded-md p-2 text-left text-sm transition duration-fast"
						class:bg-hover={index === activeIndex}
					>
						{#if IconComponent}
							<IconComponent size={14} class="shrink-0 text-faint" />
						{:else}
							<span class="w-3.5 shrink-0"></span>
						{/if}
						<span
							class="min-w-0 flex-1 truncate"
							class:text-default={index === activeIndex}
							class:text-muted={index !== activeIndex}
						>
							{#each titleSegments(match.command.title, match.highlights) as segment}
								<span class:text-action={segment.matched} class:font-medium={segment.matched}
									>{segment.text}</span
								>
							{/each}
						</span>
						<span class="shrink-0 text-2xs text-faint">{match.command.category}</span>
						{#if shortcutOf(match.command.id)}
							<span
								class="shrink-0 rounded border border-line-faint px-1 py-0.5 font-mono text-2xs text-dim"
							>
								{shortcutOf(match.command.id)}
							</span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</Card>
</div>
