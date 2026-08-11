<script lang="ts">
	import { Button, Checkbox, LoadingSpinner, ripple } from "@neoworks-dev/ui";
	import { ApplicationsFor } from "../../wailsjs/go/main/FileService";
	import type { launcher } from "../../wailsjs/go/models";
	import Dialog from "./Dialog.svelte";
	import { describeError } from "./errors";

	let {
		path,
		onOpen,
		onCancel,
	}: {
		path: string;
		onOpen: (applicationID: string, remember: boolean) => void;
		onCancel: () => void;
	} = $props();

	let applications = $state<launcher.Application[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);
	let remember = $state(false);

	$effect(() => {
		loadApplications(path);
	});

	async function loadApplications(target: string) {
		isLoading = true;
		errorMessage = "";
		try {
			applications = await ApplicationsFor(target);
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
	}
</script>

<Dialog title="Open with" onClose={onCancel}>
	{#if errorMessage}
		<p class="text-xs text-red">{errorMessage}</p>
	{:else if isLoading}
		<div class="flex justify-center p-2"><LoadingSpinner /></div>
	{:else if applications.length === 0}
		<p class="text-sm text-muted">No application is registered for this file type.</p>
	{:else}
		<div class="flex max-h-80 flex-col gap-2 overflow-y-auto">
			{#each applications as application (application.id)}
				<button
					type="button"
					use:ripple
					onclick={() => onOpen(application.id, remember)}
					class="flex min-w-0 flex-col rounded-lg p-2 text-left transition duration-fast hover:bg-hover"
				>
					<span class="truncate text-sm font-medium text-default">{application.name}</span>
					<span class="truncate text-2xs text-faint">{application.exec}</span>
				</button>
			{/each}
		</div>
		<label class="flex cursor-pointer items-center gap-2 rounded-md p-2 hover:bg-hover">
			<Checkbox checked={remember} onchange={() => (remember = !remember)} />
			<span class="text-sm text-default">Always use this for files of this type</span>
		</label>
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onclick={onCancel}>Cancel</Button>
	{/snippet}
</Dialog>
