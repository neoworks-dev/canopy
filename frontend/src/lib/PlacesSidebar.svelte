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
	import TrashIcon from "phosphor-svelte/lib/TrashIcon";
	import type { filesystem } from "../../wailsjs/go/models";

	// @neoworks-dev/ui has no sidebar: NavBar is a horizontal bottom tab bar built
	// around hrefs, and ListRow is a content row too tall for a nav rail. This
	// composes the shared Card with the design system's tokens instead.
	let {
		places,
		currentPath,
		isTrashOpen,
		onNavigate,
		onOpenTrash,
	}: {
		places: filesystem.Place[];
		currentPath: string;
		isTrashOpen: boolean;
		onNavigate: (path: string) => void;
		onOpenTrash: () => void;
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
		return !isTrashOpen && place.path === currentPath;
	}

	function rowClasses(active: boolean): string {
		if (active) {
			return "bg-raised font-medium text-default";
		}
		return "text-muted hover:bg-hover hover:text-default";
	}

	function iconWeight(active: boolean): "fill" | "regular" {
		if (active) {
			return "fill";
		}
		return "regular";
	}

	function currentMarker(active: boolean): "page" | undefined {
		if (active) {
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
			{@const active = isActive(place)}
			<button
				type="button"
				use:ripple
				onclick={() => onNavigate(place.path)}
				aria-current={currentMarker(active)}
				class="flex items-center gap-2 rounded-lg p-2 text-left text-sm transition duration-fast {rowClasses(
					active,
				)}"
			>
				<PlaceIcon size={16} weight={iconWeight(active)} class="shrink-0" />
				<span class="truncate">{place.name}</span>
			</button>
		{/each}

		<button
			type="button"
			use:ripple
			onclick={onOpenTrash}
			aria-current={currentMarker(isTrashOpen)}
			class="flex items-center gap-2 rounded-lg p-2 text-left text-sm transition duration-fast {rowClasses(
				isTrashOpen,
			)}"
		>
			<TrashIcon size={16} weight={iconWeight(isTrashOpen)} class="shrink-0" />
			<span class="truncate">Trash</span>
		</button>
	</nav>
</Card>
