<!--
	The rail beside the listing: places, devices, bookmarks, saved searches, tags
	and recent items, each a foldable section of SidebarItems. Folders in it
	take drops, so a selection can be dragged straight onto one.
-->
<script lang="ts">
	import { Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
	import {
		bookmarkSimple,
		bookmarkSimpleFill,
		desktop,
		desktopFill,
		downloadSimple,
		downloadSimpleFill,
		fileText,
		fileTextFill,
		folder,
		folderFill,
		hardDrive,
		hardDriveFill,
		hardDrives,
		hardDrivesFill,
		house,
		houseFill,
		image,
		imageFill,
		magnifyingGlass,
		magnifyingGlassFill,
		musicNote,
		musicNoteFill,
		plugsConnected,
		plugsConnectedFill,
		tag,
		tagFill,
		trash,
		trashFill,
		videoCamera,
		videoCameraFill,
	} from "../icons";
	import type { bookmarks, filesystem, recents } from "../backend/models";
	import type { DropOperation } from "../lib/dragDrop";
	import { formatFileSize } from "../lib/format";
	import { searchDescription, type SavedSearch } from "../lib/savedSearches";
	import type { SidebarSectionId } from "../lib/sidebar";
	import { colors, space, textSize } from "../theme";
	import SidebarItem from "./SidebarItem.svelte";
	import SidebarSection from "./SidebarSection.svelte";

	type IconPair = { regular: string; filled: string };

	let {
		places,
		devices,
		pinned,
		recent,
		tagNames,
		savedSearches,
		openSearchName,
		currentPath,
		isTrashOpen,
		openTag,
		width,
		collapsedSections,
		onToggleSection,
		onNavigate,
		onOpenRecent,
		onOpenTrash,
		onOpenTag,
		onRemoveBookmark,
		onForgetTag,
		onOpenSearch,
		onRenameSearch,
		onForgetSearch,
		onDropOnFolder,
		onDropInTrash,
	}: {
		places: filesystem.Place[];
		devices: filesystem.Device[];
		pinned: bookmarks.Bookmark[];
		recent: recents.Entry[];
		tagNames: string[];
		savedSearches: SavedSearch[];
		/** The saved search whose results are on screen, or "" when none is. */
		openSearchName: string;
		currentPath: string;
		isTrashOpen: boolean;
		/** The tag whose pane is on screen, or "" when none is. */
		openTag: string;
		width: number;
		collapsedSections: SidebarSectionId[];
		onToggleSection: (section: SidebarSectionId) => void;
		onNavigate: (path: string) => void;
		onOpenRecent: (entry: recents.Entry) => void;
		onOpenTrash: () => void;
		onOpenTag: (name: string) => void;
		onRemoveBookmark: (path: string) => void;
		onForgetTag: (name: string) => void;
		onOpenSearch: (search: SavedSearch) => void;
		onRenameSearch: (search: SavedSearch) => void;
		onForgetSearch: (name: string) => void;
		/** A drag landed on a folder in the rail: the same drop a pane would take. */
		onDropOnFolder: (sources: string[], destination: string, operation: DropOperation) => void;
		onDropInTrash: (sources: string[]) => void;
	} = $props();

	// The list behind this is deep enough to be a history; the sidebar offers the
	// top of it and no more.
	const visibleRecentCount = 10;

	const visibleRecent = $derived(recent.slice(0, visibleRecentCount));

	const folderIcons: IconPair = { regular: folder, filled: folderFill };

	const iconsByKind: Record<string, IconPair> = {
		home: { regular: house, filled: houseFill },
		desktop: { regular: desktop, filled: desktopFill },
		downloads: { regular: downloadSimple, filled: downloadSimpleFill },
		documents: { regular: fileText, filled: fileTextFill },
		music: { regular: musicNote, filled: musicNoteFill },
		pictures: { regular: image, filled: imageFill },
		videos: { regular: videoCamera, filled: videoCameraFill },
		root: { regular: hardDrives, filled: hardDrivesFill },
	};

	const trashIcons: IconPair = { regular: trash, filled: trashFill };
	const bookmarkIcons: IconPair = { regular: bookmarkSimple, filled: bookmarkSimpleFill };
	const searchIcons: IconPair = { regular: magnifyingGlass, filled: magnifyingGlassFill };
	const tagIcons: IconPair = { regular: tag, filled: tagFill };
	const fileIcons: IconPair = { regular: fileText, filled: fileTextFill };

	function iconFor(place: filesystem.Place): IconPair {
		const icons = iconsByKind[place.kind];
		if (!icons) {
			return folderIcons;
		}
		return icons;
	}

	function deviceIcon(device: filesystem.Device): IconPair {
		if (device.isRemovable) {
			return { regular: plugsConnected, filled: plugsConnectedFill };
		}
		return { regular: hardDrive, filled: hardDriveFill };
	}

	function recentIcon(entry: recents.Entry): IconPair {
		if (entry.isDir) {
			return folderIcons;
		}
		return fileIcons;
	}

	// Only a folder can be the location on screen; a recently opened file never
	// marks itself as current.
	function isRecentCurrent(entry: recents.Entry): boolean {
		if (!entry.isDir) {
			return false;
		}
		return isCurrent(entry.path);
	}

	function isCollapsed(section: SidebarSectionId): boolean {
		return collapsedSections.includes(section);
	}

	/** A location is current only while the listing is the pane on screen. */
	function isCurrent(path: string): boolean {
		return !isTrashOpen && openTag === "" && path === currentPath;
	}

	// A volume with no readable capacity says nothing rather than claiming to be
	// empty.
	function freeSpaceLabel(device: filesystem.Device): string {
		if (device.totalBytes === 0) {
			return "";
		}
		return `${formatFileSize(device.freeBytes)} free`;
	}

	function folderDrop(destination: string) {
		return (sources: string[], operation: DropOperation) => onDropOnFolder(sources, destination, operation);
	}

	// A recently opened file is a shortcut to that file, not a folder anything can
	// be dropped into.
	function recentDrop(entry: recents.Entry) {
		if (!entry.isDir) {
			return undefined;
		}
		return folderDrop(entry.path);
	}

	function capacityLabel(device: filesystem.Device): string {
		if (device.totalBytes === 0) {
			return device.path;
		}
		return `${device.path} · ${formatFileSize(device.freeBytes)} free of ${formatFileSize(device.totalBytes)}`;
	}
</script>

{#snippet hint(text: string)}
	<Text style={{ paddingX: space[2], paddingY: space[1], fontSize: textSize["2xs"], color: colors.faint }}>
		{text}
	</Text>
{/snippet}

<View style={{ width, shrink: 0, background: colors.surface }}>
	<Scroll style={{ flex: 1, minHeight: 0 }}>
		<View style={{ gap: space[2], paddingY: space[1.5] }}>
			<SidebarSection
				title="Places"
				isCollapsed={isCollapsed("places")}
				onToggle={() => onToggleSection("places")}
			>
				{#each places as place (place.path)}
					<SidebarItem
						icon={iconFor(place).regular}
						activeIcon={iconFor(place).filled}
						label={place.name}
						title={place.path}
						active={isCurrent(place.path)}
						onSelect={() => onNavigate(place.path)}
						onDropPaths={folderDrop(place.path)}
					/>
				{/each}
				<SidebarItem
					icon={trashIcons.regular}
					activeIcon={trashIcons.filled}
					label="Trash"
					active={isTrashOpen}
					onSelect={onOpenTrash}
					onDropPaths={(sources) => onDropInTrash(sources)}
				/>
			</SidebarSection>

			{#if devices.length > 0}
				<SidebarSection
					title="Devices"
					isCollapsed={isCollapsed("devices")}
					onToggle={() => onToggleSection("devices")}
				>
					{#each devices as device (device.path)}
						<SidebarItem
							icon={deviceIcon(device).regular}
							activeIcon={deviceIcon(device).filled}
							label={device.name}
							detail={freeSpaceLabel(device)}
							title={capacityLabel(device)}
							active={isCurrent(device.path)}
							onSelect={() => onNavigate(device.path)}
							onDropPaths={folderDrop(device.path)}
						/>
					{/each}
				</SidebarSection>
			{/if}

			<SidebarSection
				title="Bookmarks"
				isCollapsed={isCollapsed("bookmarks")}
				onToggle={() => onToggleSection("bookmarks")}
			>
				{#each pinned as bookmark (bookmark.path)}
					<SidebarItem
						icon={bookmarkIcons.regular}
						activeIcon={bookmarkIcons.filled}
						label={bookmark.name}
						title={bookmark.path}
						active={isCurrent(bookmark.path)}
						onSelect={() => onNavigate(bookmark.path)}
						onDropPaths={folderDrop(bookmark.path)}
						onRemove={() => onRemoveBookmark(bookmark.path)}
						removeLabel="Unpin {bookmark.name}"
					/>
				{/each}
				{#if pinned.length === 0}
					{@render hint("Pin a folder to keep it here")}
				{/if}
			</SidebarSection>

			<SidebarSection
				title="Searches"
				isCollapsed={isCollapsed("searches")}
				onToggle={() => onToggleSection("searches")}
			>
				{#each savedSearches as search (search.name)}
					<SidebarItem
						icon={searchIcons.regular}
						activeIcon={searchIcons.filled}
						label={search.name}
						title={searchDescription(search)}
						active={openSearchName === search.name}
						onSelect={() => onOpenSearch(search)}
						onEdit={() => onRenameSearch(search)}
						editLabel="Rename {search.name}"
						onRemove={() => onForgetSearch(search.name)}
						removeLabel="Remove {search.name} from the sidebar"
					/>
				{/each}
				{#if savedSearches.length === 0}
					{@render hint("Save a search to keep it here")}
				{/if}
			</SidebarSection>

			<SidebarSection
				title="Tags"
				isCollapsed={isCollapsed("tags")}
				onToggle={() => onToggleSection("tags")}
			>
				{#each tagNames as name (name)}
					<SidebarItem
						icon={tagIcons.regular}
						activeIcon={tagIcons.filled}
						label={name}
						active={openTag === name}
						onSelect={() => onOpenTag(name)}
						onRemove={() => onForgetTag(name)}
						removeLabel="Remove {name} from the sidebar"
					/>
				{/each}
				{#if tagNames.length === 0}
					{@render hint("Tag a file to keep it here")}
				{/if}
			</SidebarSection>

			<SidebarSection
				title="Recent"
				isCollapsed={isCollapsed("recent")}
				onToggle={() => onToggleSection("recent")}
			>
				{#each visibleRecent as entry (entry.path)}
					<SidebarItem
						icon={recentIcon(entry).regular}
						activeIcon={recentIcon(entry).filled}
						label={entry.name}
						title={entry.path}
						active={isRecentCurrent(entry)}
						onSelect={() => onOpenRecent(entry)}
						onDropPaths={recentDrop(entry)}
					/>
				{/each}
				{#if visibleRecent.length === 0}
					{@render hint("Nothing opened yet")}
				{/if}
			</SidebarSection>
		</View>
	</Scroll>
</View>
