<script lang="ts">
	import { Button, Checkbox } from "@neoworks-dev/ui";
	import Dialog from "./Dialog.svelte";
	import type { ConflictPrompt, ConflictResolution } from "./fileops";

	let {
		prompt,
		onResolve,
	}: {
		prompt: ConflictPrompt;
		onResolve: (resolution: ConflictResolution, applyToAll: boolean) => void;
	} = $props();

	let applyToAll = $state(false);

	const name = $derived(nameOf(prompt.destinationPath));

	function resolve(resolution: ConflictResolution) {
		onResolve(resolution, applyToAll);
	}

	function cancel() {
		onResolve("cancel", false);
	}

	function nameOf(path: string): string {
		const parts = path.split("/");
		return parts[parts.length - 1];
	}
</script>

<Dialog title="{name} already exists" onClose={cancel}>
	<p class="text-sm text-muted">
		Something is already at the destination. Choose what happens to the incoming item.
	</p>
	<dl class="flex flex-col gap-2 rounded-lg bg-raised p-2">
		<div class="min-w-0">
			<dt class="text-2xs tracking-caps text-faint uppercase">Incoming</dt>
			<dd class="truncate text-xs text-muted">{prompt.sourcePath}</dd>
		</div>
		<div class="min-w-0">
			<dt class="text-2xs tracking-caps text-faint uppercase">Existing</dt>
			<dd class="truncate text-xs text-muted">{prompt.destinationPath}</dd>
		</div>
	</dl>
	<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
		<Checkbox checked={applyToAll} onchange={() => (applyToAll = !applyToAll)} />
		<span class="text-sm text-default">Apply to all remaining conflicts</span>
	</label>

	{#snippet footer()}
		<Button size="sm" variant="ghost" onclick={cancel}>Cancel</Button>
		<Button size="sm" variant="ghost" onclick={() => resolve("skip")}>Skip</Button>
		<Button size="sm" variant="surface" onclick={() => resolve("keepBoth")}>Keep both</Button>
		<Button size="sm" variant="surface" onclick={() => resolve("replace")}>Replace</Button>
	{/snippet}
</Dialog>
