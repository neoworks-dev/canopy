<!--
	Lists the applications registered for a file's type and opens it with the
	one picked, optionally remembering the choice for every file of that type.
-->
<script lang="ts">
	import { Button as PlainButton, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
	import { ApplicationsFor } from "../backend/FileService";
	import type { launcher } from "../backend/models";
	import { describeError } from "../lib/errors";
	import { colors, duration, radius, space, textSize } from "../theme";
	import CheckRow from "./CheckRow.svelte";
	import Dialog from "./Dialog.svelte";
	import Button from "./design/Button.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";

	const listMaxHeight = 320;

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
		<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
	{:else if isLoading}
		<View style={{ align: "center", padding: space[2] }}><LoadingSpinner /></View>
	{:else if applications.length === 0}
		<Text style={{ fontSize: textSize.sm, color: colors.muted }}>
			No application is registered for this file type.
		</Text>
	{:else}
		<Scroll style={{ maxHeight: listMaxHeight }}>
			<View style={{ gap: space[2] }}>
				{#each applications as application (application.id)}
					<PlainButton
						style={{
							padding: space[2],
							radius: radius.lg,
							transition: duration.fast,
						}}
						hover={{ background: colors.hover }}
						onpress={() => onOpen(application.id, remember)}
					>
						<View style={{ flex: 1, minWidth: 0 }}>
							<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
								{application.name}
							</Text>
							<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>
								{application.exec}
							</Text>
						</View>
					</PlainButton>
				{/each}
			</View>
		</Scroll>
		<CheckRow bind:checked={remember} label="Always use this for files of this type" />
	{/if}

	{#snippet footer()}
		<Button size="sm" variant="ghost" onpress={onCancel}>Cancel</Button>
	{/snippet}
</Dialog>
