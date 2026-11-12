<!--
	Yazi's status bar along the bottom edge: the mode in a coloured block, the
	hovered entry's size and name, then what the selection or an error has to
	say; on the right the permissions, the branch and where the cursor is.
-->
<script lang="ts">
	import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
	import { gitBranch, warning } from "../icons";
	import { formatFileSize } from "../lib/format";
	import type { PaneStatus } from "../lib/paneStatus";
	import {
		modeLabel,
		percentLabel,
		permissionCharacters,
		positionLabel,
		type PermissionTone,
	} from "../lib/statusBar";
	import { colors, radius, space, textSize } from "../theme";

	const statusBarHeight = 24;

	let {
		status,
		summary,
		errorMessage,
		onDismissError,
	}: {
		status: PaneStatus;
		/** What the selection adds up to, when more than the hovered entry is selected. */
		summary: string;
		errorMessage: string;
		onDismissError: () => void;
	} = $props();

	const tonesToColors: Record<PermissionTone, string> = {
		type: colors.blue,
		read: colors.amber,
		write: colors.red,
		execute: colors.green,
		none: colors.faint,
	};

	const modeColors: Record<PaneStatus["mode"], string> = {
		normal: colors.blue,
		select: colors.amber,
		unset: colors.red,
	};

	function hoveredSize(): string {
		const hovered = status.hovered;
		if (!hovered || hovered.isDir) {
			return "";
		}
		return formatFileSize(hovered.size);
	}

	function hoveredMode(): string {
		const hovered = status.hovered;
		if (!hovered) {
			return "";
		}
		return hovered.mode;
	}

	function hoveredName(): string {
		const hovered = status.hovered;
		if (!hovered) {
			return "";
		}
		return hovered.name;
	}
</script>

<View
	style={{
		direction: "row",
		height: statusBarHeight,
		shrink: 0,
		align: "center",
		gap: space[2],
		paddingX: space[2],
		borderTopWidth: 1,
		borderColor: colors.line,
		background: colors.surface,
		fontSize: textSize.xs,
	}}
>
	<View style={{ shrink: 0, paddingX: space[1.5], paddingY: 1, radius: radius.xs, background: modeColors[status.mode] }}>
		<Text style={{ fontSize: textSize["2xs"], fontWeight: "bold", color: colors.inverse }}>{modeLabel(status.mode)}</Text>
	</View>
	{#if hoveredSize()}
		<Text style={{ shrink: 0, color: colors.muted }}>{hoveredSize()}</Text>
	{/if}
	<Text style={{ minWidth: 0, shrink: 1, truncate: true, color: colors.text }}>{hoveredName()}</Text>

	{#if errorMessage}
		<Button style={{ direction: "row", minWidth: 0, align: "center", gap: space[1] }} onpress={onDismissError}>
			<Icon svg={warning} size={12} color={colors.red} />
			<Text style={{ minWidth: 0, truncate: true, color: colors.red }}>{errorMessage}</Text>
		</Button>
	{:else if summary}
		<Text style={{ minWidth: 0, truncate: true, color: colors.amber }}>{summary}</Text>
	{/if}

	<View style={{ flex: 1 }} />

	<Text style={{ shrink: 0, fontFamily: "mono" }}>
		{#each permissionCharacters(hoveredMode()) as part, index (index)}
			<Text style={{ color: tonesToColors[part.tone] }}>{part.character}</Text>
		{/each}
	</Text>
	{#if status.branch}
		<View style={{ direction: "row", shrink: 0, align: "center", gap: space[1], color: colors.dim }}>
			<Icon svg={gitBranch} size={12} />
			<Text style={{ maxWidth: 160, truncate: true }}>{status.branch}</Text>
		</View>
	{/if}
	<Text style={{ shrink: 0, color: colors.muted }}>{positionLabel(status.cursorIndex, status.entryCount)}</Text>
	<Text style={{ shrink: 0, width: 36, textAlign: "right", color: colors.dim }}>
		{percentLabel(status.cursorIndex, status.entryCount)}
	</Text>
</View>
