<!--
	What is in the trash, each item with where it came from and a button that
	puts it back, and a button that empties the lot after confirming. It fills
	the space it is given and scrolls its items.
-->
<script lang="ts">
	import { List, Text, View } from "@neoworks/svelte-gpui/ui";
	import { arrowUUpLeft, trash } from "../icons";
	import { EmptyTrash, ListTrash, RestoreFromTrash } from "../backend/FileService";
	import type { fileops } from "../backend/models";
	import { describeError } from "../lib/errors";
	import { formatFileSize, formatModifiedTime } from "../lib/format";
	import { colors, duration, radius, space, textSize } from "../theme";
	import ConfirmDialog from "./ConfirmDialog.svelte";
	import IconButton from "./IconButton.svelte";
	import Button from "./design/Button.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";
	import SectionHeader from "./design/SectionHeader.svelte";

	let { onRestored }: { onRestored: (path: string) => void } = $props();

	let items = $state.raw<fileops.TrashedItem[]>([]);
	let errorMessage = $state("");
	let isLoading = $state(true);
	let isConfirmingEmpty = $state(false);

	$effect(() => {
		reload();
	});

	async function reload() {
		isLoading = true;
		errorMessage = "";
		try {
			items = await ListTrash();
		} catch (error) {
			errorMessage = describeError(error);
		}
		isLoading = false;
	}

	async function restore(item: fileops.TrashedItem) {
		try {
			const restoredPath = await RestoreFromTrash(item.trashPath);
			onRestored(restoredPath);
		} catch (error) {
			errorMessage = describeError(error);
		}
		reload();
	}

	async function confirmEmpty() {
		isConfirmingEmpty = false;
		try {
			await EmptyTrash();
		} catch (error) {
			errorMessage = describeError(error);
		}
		reload();
	}

	function sizeLabel(item: fileops.TrashedItem): string {
		if (item.isDir) {
			return "—";
		}
		return formatFileSize(item.size);
	}
</script>

<View style={{ flex: 1, minHeight: 0, gap: space[2] }}>
	<View style={{ direction: "row", align: "center", gap: space[2] }}>
		<SectionHeader title="Trash" />
		<View style={{ flex: 1 }} />
		<Button
			size="sm"
			variant="ghost"
			icon={trash}
			disabled={items.length === 0}
			onpress={() => (isConfirmingEmpty = true)}
		>
			Empty
		</Button>
	</View>

	{#if errorMessage}
		<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
	{/if}

	{#if isLoading}
		<View style={{ align: "center", padding: space[2] }}><LoadingSpinner /></View>
	{:else if items.length === 0}
		<Text style={{ fontSize: textSize.sm, color: colors.dim }}>The trash is empty.</Text>
	{:else}
		<List items={items} key={(item) => item.trashPath} style={{ flex: 1, minHeight: 0 }}>
			{#snippet row(item)}
				<View style={{ paddingBottom: space[2] }}>
					<View
						style={{
							direction: "row",
							align: "center",
							gap: space[2],
							padding: space[2],
							radius: radius.lg,
							transition: duration.fast,
						}}
						hover={{ background: colors.hover }}
					>
						<View style={{ flex: 1, minWidth: 0 }}>
							<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
								{item.name}
							</Text>
							<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>
								{item.originalPath}
							</Text>
						</View>
						<Text style={{ shrink: 0, fontSize: textSize.xs, color: colors.dim }}>{sizeLabel(item)}</Text>
						<Text style={{ shrink: 0, fontSize: textSize.xs, color: colors.dim }}>
							{formatModifiedTime(item.deletedAt)}
						</Text>
						<IconButton icon={arrowUUpLeft} size={16} title="Restore" onpress={() => restore(item)} />
					</View>
				</View>
			{/snippet}
		</List>
	{/if}
</View>

{#if isConfirmingEmpty}
	<ConfirmDialog
		title="Empty the trash?"
		message="Every item in the trash is deleted permanently. This cannot be undone."
		confirmLabel="Empty trash"
		onConfirm={confirmEmpty}
		onCancel={() => (isConfirmingEmpty = false)}
	/>
{/if}
