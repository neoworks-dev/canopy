<script lang="ts">
	import { Card, SectionHeader, ripple } from "@neoworks-dev/ui";
	import type { IconComponent } from "@neoworks-dev/ui";
	import HouseIcon from "phosphor-svelte/lib/HouseIcon";
	import DesktopIcon from "phosphor-svelte/lib/DesktopIcon";
	import DownloadSimpleIcon from "phosphor-svelte/lib/DownloadSimpleIcon";
	import FileTextIcon from "phosphor-svelte/lib/FileTextIcon";
	import MusicNoteIcon from "phosphor-svelte/lib/MusicNoteIcon";
	import ImageIcon from "phosphor-svelte/lib/ImageIcon";
	import VideoCameraIcon from "phosphor-svelte/lib/VideoCameraIcon";
	import HardDrivesIcon from "phosphor-svelte/lib/HardDrivesIcon";
	import FolderIcon from "phosphor-svelte/lib/FolderIcon";
	import type { filesystem } from "../../wailsjs/go/models";

	// @neoworks-dev/ui has no sidebar: NavBar is a horizontal bottom tab bar built
	// around hrefs, and ListRow is a content row too tall for a nav rail. This
	// composes the shared Card with the design system's tokens instead.
	let {
		places,
		currentPath,
		onNavigate,
	}: {
		places: filesystem.Place[];
		currentPath: string;
		onNavigate: (path: string) => void;
	} = $props();

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

	function isActive(place: filesystem.Place): boolean {
		return place.path === currentPath;
	}

	function rowClasses(place: filesystem.Place): string {
		if (isActive(place)) {
			return "bg-raised font-medium text-default";
		}
		return "text-muted hover:bg-hover hover:text-default";
	}

	function iconWeight(place: filesystem.Place): "fill" | "regular" {
		if (isActive(place)) {
			return "fill";
		}
		return "regular";
	}

	function currentMarker(place: filesystem.Place): "page" | undefined {
		if (isActive(place)) {
			return "page";
		}
		return undefined;
	}
</script>

<Card
	surface="elevated"
	padding="none"
	class="flex w-52 shrink-0 flex-col gap-2 overflow-y-auto p-2 shadow-lg"
>
	<SectionHeader title="Places" />

	<nav class="flex flex-col gap-2">
		{#each places as place (place.path)}
			{@const PlaceIcon = iconFor(place)}
			<button
				type="button"
				use:ripple
				onclick={() => onNavigate(place.path)}
				aria-current={currentMarker(place)}
				class="flex items-center gap-2 rounded-lg p-2 text-left text-sm transition duration-fast {rowClasses(
					place,
				)}"
			>
				<PlaceIcon size={16} weight={iconWeight(place)} class="shrink-0" />
				<span class="truncate">{place.name}</span>
			</button>
		{/each}
	</nav>
</Card>
