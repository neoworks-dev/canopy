<!--
	What an archive holds, one level at a time and without unpacking it. It fills
	the space it is given and scrolls its rows.
-->
<script lang="ts">
    import { Button, Icon, List, Text, View } from "@neoworks/svelte-gpui/ui";
    import { ListArchive } from "../backend/FileService";
    import { caretRight, file, folder } from "../icons";
    import {
        insideCrumbs,
        joinInside,
        parentInside,
        rowsInside,
        type ArchiveMember,
        type ArchiveRow,
    } from "../lib/archive";
    import { describeError } from "../lib/errors";
    import { formatFileSize, formatModifiedTime } from "../lib/format";
    import { colors, duration, radius, space, textSize } from "../theme";
    import DesignButton from "./design/Button.svelte";
    import LoadingSpinner from "./design/LoadingSpinner.svelte";
    import SectionHeader from "./design/SectionHeader.svelte";

    const rowHeight = 44;

    /** A line of the listing: the way back up, or what the archive holds at this level. */
    type Line = { kind: "parent" } | { kind: "row"; row: ArchiveRow };

    let {
        path,
        inside,
        onDescend,
        onExtract,
    }: {
        path: string;
        /** The level inside the archive being looked at, empty at the top. */
        inside: string;
        onDescend: (inside: string) => void;
        onExtract: () => void;
    } = $props();

    let members = $state.raw<ArchiveMember[]>([]);
    let errorMessage = $state("");
    let isLoading = $state(false);

    // Guards against a slow listing of an archive this pane has already left
    // landing after the one it is showing now.
    let latestRequest = 0;

    const rows = $derived(rowsInside(members, inside));
    const crumbs = $derived(insideCrumbs(inside));
    const lines = $derived(linesFor(rows, inside));

    $effect(() => {
        read(path);
    });

    async function read(archivePath: string) {
        const request = ++latestRequest;
        isLoading = true;
        errorMessage = "";
        try {
            apply(request, await ListArchive(archivePath), "");
        } catch (error) {
            apply(request, [], describeError(error));
        }
    }

    function apply(request: number, listed: ArchiveMember[], message: string) {
        if (request !== latestRequest) {
            return;
        }
        members = listed;
        errorMessage = message;
        isLoading = false;
    }

    function linesFor(shown: ArchiveRow[], level: string): Line[] {
        const found: Line[] = shown.map((row) => ({ kind: "row", row }));
        if (level === "") {
            return found;
        }
        return [{ kind: "parent" }, ...found];
    }

    // A folder opens on the second press, as it does in the listing.
    function press(line: Line, clicks: number) {
        if (clicks < 2) {
            return;
        }
        if (line.kind === "parent") {
            onDescend(parentInside(inside));
            return;
        }
        if (line.row.isDir) {
            onDescend(joinInside(inside, line.row.label));
        }
    }

    function sizeLabel(row: ArchiveRow): string {
        if (row.isDir) {
            return "—";
        }
        return formatFileSize(row.size);
    }

    function lineKey(line: Line): string {
        if (line.kind === "parent") {
            return "..";
        }
        return line.row.label;
    }

    const crumbLook = {
        paddingX: space[1],
        radius: radius.xs,
        color: colors.dim,
        transition: duration.fast,
    } as const;
</script>

<View style={{ flex: 1, minHeight: 0, gap: space[2] }}>
    <View style={{ direction: "row", align: "center", gap: space[2] }}>
        <SectionHeader title="Archive" />
        <Text
            style={{
                flex: 1,
                minWidth: 0,
                truncate: true,
                fontSize: textSize.xs,
                color: colors.faint,
            }}>{path}</Text
        >
        <DesignButton size="sm" variant="ghost" onpress={onExtract}
            >Extract all</DesignButton
        >
    </View>

    <View
        style={{
            direction: "row",
            wrap: true,
            align: "center",
            gap: space[1],
            fontSize: textSize.xs,
        }}
    >
        <Button
            style={crumbLook}
            hover={{ background: colors.hover }}
            onpress={() => onDescend("")}>Top</Button
        >
        {#each crumbs as crumb (crumb.inside)}
            <Icon svg={caretRight} size={12} color={colors.faint} />
            <Button
                style={crumbLook}
                hover={{ background: colors.hover }}
                onpress={() => onDescend(crumb.inside)}
            >
                {crumb.label}
            </Button>
        {/each}
    </View>

    {#if errorMessage}
        <Text style={{ fontSize: textSize.xs, color: colors.red }}
            >{errorMessage}</Text
        >
    {:else if isLoading}
        <View style={{ align: "center", padding: space[2] }}
            ><LoadingSpinner /></View
        >
    {:else if rows.length === 0}
        <Text style={{ fontSize: textSize.sm, color: colors.dim }}
            >There is nothing in this archive.</Text
        >
    {:else}
        <List
            items={lines}
            {rowHeight}
            key={lineKey}
            style={{ flex: 1, minHeight: 0 }}
        >
            {#snippet row(line)}
                <Button
                    style={{
                        flex: 1,
                        align: "center",
                        gap: space[2],
                        padding: space[2],
                        radius: radius.lg,
                        transition: duration.fast,
                    }}
                    hover={{ background: colors.hover }}
                    onpress={(event) => press(line, event.clicks)}
                >
                    {#if line.kind === "parent"}
                        <Icon
                            svg={folder}
                            size={20}
                            color={colors.blue}
                            style={{ shrink: 0 }}
                        />
                        <Text
                            style={{ fontSize: textSize.sm, color: colors.dim }}
                            >..</Text
                        >
                    {:else}
                        {#if line.row.isDir}
                            <Icon
                                svg={folder}
                                size={20}
                                color={colors.blue}
                                style={{ shrink: 0 }}
                            />
                        {:else}
                            <Icon
                                svg={file}
                                size={20}
                                color={colors.dim}
                                style={{ shrink: 0 }}
                            />
                        {/if}
                        <Text
                            style={{
                                flex: 1,
                                minWidth: 0,
                                truncate: true,
                                fontSize: textSize.sm,
                                color: colors.text,
                            }}
                        >
                            {line.row.label}
                        </Text>
                        <Text
                            style={{
                                shrink: 0,
                                fontSize: textSize.xs,
                                color: colors.dim,
                            }}>{sizeLabel(line.row)}</Text
                        >
                        <Text
                            style={{
                                shrink: 0,
                                fontSize: textSize.xs,
                                color: colors.dim,
                            }}
                        >
                            {formatModifiedTime(line.row.modTime)}
                        </Text>
                    {/if}
                </Button>
            {/snippet}
        </List>
    {/if}
</View>
