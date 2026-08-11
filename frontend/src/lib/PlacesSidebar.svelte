<script lang="ts">
	import { Card } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import DesktopIcon from "phosphor-svelte/lib/DesktopIcon";
	import DownloadSimpleIcon from "phosphor-svelte/lib/DownloadSimpleIcon";
	import FileTextIcon from "phosphor-svelte/lib/FileTextIcon";
	import MusicNoteIcon from "phosphor-svelte/lib/MusicNoteIcon";
	import ImageIcon from "phosphor-svelte/lib/ImageIcon";
	import VideoCameraIcon from "phosphor-svelte/lib/VideoCameraIcon";
	import HardDrivesIcon from "phosphor-svelte/lib/HardDrivesIcon";
	import HardDriveIcon from "phosphor-svelte/lib/HardDriveIcon";
	import FolderIcon from "phosphor-svelte/lib/FolderIcon";
	import TrashIcon from "phosphor-svelte/lib/TrashIcon";
	import BookmarkSimpleIcon from "phosphor-svelte/lib/BookmarkSimpleIcon";
	import PlugsConnectedIcon from "phosphor-svelte/lib/PlugsConnectedIcon";
	import TagIcon from "phosphor-svelte/lib/TagIcon";
	import type { bookmarks, filesystem, recents } from "../../wailsjs/go/models";
	import { formatFileSize } from "./format";
	import SidebarItem from "./SidebarItem.svelte";
	import SidebarSection from "./SidebarSection.svelte";
	import type { SidebarSectionId } from "./sidebar";

	let {
		places,
		devices,
		pinned,
		recent,
		tagNames,
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
	}: {
		places: filesystem.Place[];
		devices: filesystem.Device[];
		pinned: bookmarks.Bookmark[];
		recent: recents.Entry[];
		tagNames: string[];
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
	} = $props();

	// The list behind this is deep enough to be a history; the sidebar offers the
	// top of it and no more.
	const visibleRecentCount = 10;

	const visibleRecent = $derived(recent.slice(0, visibleRecentCount));

	const iconsByKind: Record<string, IconComponent> = {
		home: HouseIcon,
		desktop: DesktopIcon,
		downloads: DownloadSimpleIcon,
		documents: FileTextIcon,
		music: MusicNoteIcon,
		pictures: ImageIcon,
		videos: VideoCameraIcon,
		root: HardDrivesIcon,
	};

	function iconFor(place: filesystem.Place): IconComponent {
		const icon = iconsByKind[place.kind];
		if (!icon) {
			return FolderIcon;
		}
		return icon;
	}

	function deviceIcon(device: filesystem.Device): IconComponent {
		if (device.isRemovable) {
			return PlugsConnectedIcon;
		}
		return HardDriveIcon;
	}

	function recentIcon(entry: recents.Entry): IconComponent {
		if (entry.isDir) {
			return FolderIcon;
		}
		return FileTextIcon;
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

	function capacityLabel(device: filesystem.Device): string {
		if (device.totalBytes === 0) {
			return device.path;
		}
		return `${device.path} · ${formatFileSize(device.freeBytes)} free of ${formatFileSize(device.totalBytes)}`;
	}
</script>

<div class="flex shrink-0 flex-col" style="width: {width}px">
	<Card
		surface="elevated"
		padding="none"
		class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-2 shadow-lg"
	>
		<SidebarSection
			title="Places"
			isCollapsed={isCollapsed("places")}
			onToggle={() => onToggleSection("places")}
		>
			{#each places as place (place.path)}
				<SidebarItem
					icon={iconFor(place)}
					label={place.name}
					title={place.path}
					active={isCurrent(place.path)}
					onSelect={() => onNavigate(place.path)}
				/>
			{/each}
			<SidebarItem
				icon={TrashIcon}
				label="Trash"
				active={isTrashOpen}
				onSelect={onOpenTrash}
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
						icon={deviceIcon(device)}
						label={device.name}
						detail={freeSpaceLabel(device)}
						title={capacityLabel(device)}
						active={isCurrent(device.path)}
						onSelect={() => onNavigate(device.path)}
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
					icon={BookmarkSimpleIcon}
					label={bookmark.name}
					title={bookmark.path}
					active={isCurrent(bookmark.path)}
					onSelect={() => onNavigate(bookmark.path)}
					onRemove={() => onRemoveBookmark(bookmark.path)}
					removeLabel="Unpin {bookmark.name}"
				/>
			{/each}
			{#if pinned.length === 0}
				<p class="px-2 py-1 text-2xs text-faint">Pin a folder to keep it here</p>
			{/if}
		</SidebarSection>

		<SidebarSection
			title="Tags"
			isCollapsed={isCollapsed("tags")}
			onToggle={() => onToggleSection("tags")}
		>
			{#each tagNames as name (name)}
				<SidebarItem
					icon={TagIcon}
					label={name}
					active={openTag === name}
					onSelect={() => onOpenTag(name)}
					onRemove={() => onForgetTag(name)}
					removeLabel="Remove {name} from the sidebar"
				/>
			{/each}
			{#if tagNames.length === 0}
				<p class="px-2 py-1 text-2xs text-faint">Tag a file to keep it here</p>
			{/if}
		</SidebarSection>

		<SidebarSection
			title="Recent"
			isCollapsed={isCollapsed("recent")}
			onToggle={() => onToggleSection("recent")}
		>
			{#each visibleRecent as entry (entry.path)}
				<SidebarItem
					icon={recentIcon(entry)}
					label={entry.name}
					title={entry.path}
					active={isRecentCurrent(entry)}
					onSelect={() => onOpenRecent(entry)}
				/>
			{/each}
			{#if visibleRecent.length === 0}
				<p class="px-2 py-1 text-2xs text-faint">Nothing opened yet</p>
			{/if}
		</SidebarSection>
	</Card>
</div>
