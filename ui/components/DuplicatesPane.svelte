<!--
	Finds files holding the same bytes under a folder. The search runs in the
	backend and reports progress and findings as events; each group of copies can
	be opened, shown in its folder or trashed, keeping the first. It fills the
	space it is given and scrolls its groups.
-->
<script lang="ts">
	import { List, Text, View } from "@neoworks/svelte-gpui/ui";
	import { arrowSquareOut, trash, xCircle } from "../icons";
	import { CancelDuplicateSearch, FindDuplicates, StartOperation } from "../backend/FileService";
	import { EventsOn } from "../backend/events";
	import {
		duplicateEvents,
		extraPathsOf,
		groupLabel,
		recoverableLabel,
		remainingGroups,
		searchProgressLabel,
		type DuplicateGroup,
		type DuplicateReport,
	} from "../lib/duplicates";
	import { describeError } from "../lib/errors";
	import { parentDirectoryOf } from "../lib/format";
	import { colors, duration, radius, space, textSize } from "../theme";
	import IconButton from "./IconButton.svelte";
	import Button from "./design/Button.svelte";
	import LoadingSpinner from "./design/LoadingSpinner.svelte";
	import SectionHeader from "./design/SectionHeader.svelte";

	let {
		path,
		onOpen,
		onReveal,
		onChanged,
	}: {
		path: string;
		onOpen: (path: string) => void;
		/** Shows a copy where it lives, which moves this pane to its folder. */
		onReveal: (path: string) => void;
		/** Files were trashed from here, which the undo label follows. */
		onChanged: () => void;
	} = $props();

	let report = $state.raw<DuplicateReport | null>(null);
	let errorMessage = $state("");
	let isSearching = $state(false);
	let trashed = $state(new Set<string>());

	// The search the pane is following, and the counter that tells a search
	// started for a folder it has already left.
	let searchID = "";
	let latestSearch = 0;

	// A small folder can be searched through before the call that started it has
	// even reported its id, so a report that arrives that early waits here until
	// there is an id to match it against.
	let earlyReport: { update: DuplicateReport; running: boolean } | null = null;

	const groups = $derived(remainingGroups(foundGroups(), trashed));
	const finishedWithSavings = $derived(report !== null && report.stage === "done" && report.recoverableBytes > 0);

	$effect(() => {
		startSearch(path);
		return stopSearch;
	});

	$effect(() => {
		const stops = [
			EventsOn(duplicateEvents.progress, (update: DuplicateReport) => apply(update, true)),
			EventsOn(duplicateEvents.finished, (update: DuplicateReport) => apply(update, false)),
		];
		return () => stops.forEach((stop) => stop());
	});

	async function startSearch(root: string) {
		const request = ++latestSearch;
		report = null;
		earlyReport = null;
		trashed = new Set();
		errorMessage = "";
		isSearching = true;
		try {
			adopt(request, await FindDuplicates(root));
		} catch (error) {
			errorMessage = describeError(error);
			isSearching = false;
		}
	}

	// A search whose folder is already gone by the time it reports its id has
	// nobody to search for, so it is called off rather than followed.
	function adopt(request: number, started: string) {
		if (request !== latestSearch) {
			cancelSearch(started);
			return;
		}
		searchID = started;
		applyEarlyReport(started);
	}

	function applyEarlyReport(started: string) {
		const waiting = earlyReport;
		earlyReport = null;
		if (!waiting || waiting.update.id !== started) {
			return;
		}
		apply(waiting.update, waiting.running);
	}

	function stopSearch() {
		latestSearch++;
		cancelSearch(searchID);
		searchID = "";
		isSearching = false;
	}

	// A search that has already finished is unknown to the scanner, which is
	// nothing the pane has to react to.
	function cancelSearch(id: string) {
		if (!id) {
			return;
		}
		CancelDuplicateSearch(id).catch(() => {});
	}

	function apply(update: DuplicateReport, running: boolean) {
		if (!searchID) {
			earlyReport = { update, running };
			return;
		}
		if (update.id !== searchID) {
			return;
		}
		report = update;
		isSearching = running;
	}

	function foundGroups(): DuplicateGroup[] {
		if (!report || !report.groups) {
			return [];
		}
		return report.groups;
	}

	async function trashFiles(paths: string[]) {
		if (paths.length === 0) {
			return;
		}
		try {
			await StartOperation("trash", paths, "");
		} catch (error) {
			errorMessage = describeError(error);
			return;
		}
		trashed = new Set([...trashed, ...paths]);
		onChanged();
	}

	function nameOf(filePath: string): string {
		const name = filePath.split("/").pop();
		if (!name) {
			return filePath;
		}
		return name;
	}

	function progressLine(): string {
		const progress = searchProgressLabel(report);
		if (report && finishedWithSavings) {
			return `${progress} · ${recoverableLabel(report)}`;
		}
		return progress;
	}
</script>

<View style={{ flex: 1, minHeight: 0, gap: space[2] }}>
	<View style={{ direction: "row", align: "center", gap: space[2] }}>
		<SectionHeader title="Duplicates" />
		<Text style={{ flex: 1, minWidth: 0, truncate: true, paddingBottom: space[2], fontSize: textSize.xs, color: colors.faint }}>
			{path}
		</Text>
		{#if isSearching}
			<Button size="sm" variant="ghost" icon={xCircle} onpress={stopSearch}>Stop</Button>
		{/if}
	</View>

	<Text style={{ fontSize: textSize.xs, color: colors.dim }}>{progressLine()}</Text>

	{#if errorMessage}
		<Text style={{ fontSize: textSize.xs, color: colors.red }}>{errorMessage}</Text>
	{/if}

	{#if isSearching && groups.length === 0}
		<View style={{ align: "center", padding: space[2] }}><LoadingSpinner /></View>
	{/if}

	<List items={groups} key={(group) => group.paths[0]} style={{ flex: 1, minHeight: 0 }}>
		{#snippet row(group)}
			<View style={{ paddingBottom: space[2] }}>
				<View style={{ gap: space[1], padding: space[2], radius: radius.lg, background: colors.raised }}>
					<View style={{ direction: "row", align: "center", gap: space[2] }}>
						<Text style={{ flex: 1, minWidth: 0, truncate: true, fontSize: textSize.xs, color: colors.dim }}>
							{groupLabel(group)}
						</Text>
						<Button size="sm" variant="ghost" icon={trash} onpress={() => trashFiles(extraPathsOf(group))}>
							Keep the first
						</Button>
					</View>
					{#each group.paths as filePath (filePath)}
						<View
							style={{
								direction: "row",
								align: "center",
								gap: space[2],
								padding: space[1],
								radius: radius.lg,
								transition: duration.fast,
							}}
							hover={{ background: colors.hover }}
						>
							<View style={{ flex: 1, minWidth: 0 }} onpress={() => onOpen(filePath)}>
								<Text style={{ truncate: true, fontSize: textSize.sm, fontWeight: "medium", color: colors.text }}>
									{nameOf(filePath)}
								</Text>
								<Text style={{ truncate: true, fontSize: textSize["2xs"], color: colors.faint }}>
									{parentDirectoryOf(filePath)}
								</Text>
							</View>
							<IconButton icon={arrowSquareOut} size={16} title="Show in folder" onpress={() => onReveal(filePath)} />
							<IconButton icon={trash} size={16} title="Move to trash" onpress={() => trashFiles([filePath])} />
						</View>
					{/each}
				</View>
			</View>
		{/snippet}
	</List>
</View>
