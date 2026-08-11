/**
 * Which groups the sidebar shows and which of them are folded away. Kept free
 * of the DOM so the stored state can be checked the same way every other
 * setting is.
 */
export type SidebarSectionId = "places" | "devices" | "bookmarks" | "tags" | "recent";

export const sidebarSectionIds: SidebarSectionId[] = [
	"places",
	"devices",
	"bookmarks",
	"tags",
	"recent",
];

/** Folding a group open or shut, as the list of the ones that are shut. */
export function sectionsAfterToggle(
	collapsed: SidebarSectionId[],
	section: SidebarSectionId,
): SidebarSectionId[] {
	if (collapsed.includes(section)) {
		return collapsed.filter((folded) => folded !== section);
	}
	return [...collapsed, section];
}

/** A stored list with names from another version drops the ones it cannot use. */
export function sidebarSectionsFrom(stored: string[] | undefined): SidebarSectionId[] {
	if (!Array.isArray(stored)) {
		return [];
	}
	return stored.filter(isSidebarSectionId);
}

function isSidebarSectionId(value: string): value is SidebarSectionId {
	return sidebarSectionIds.some((section) => section === value);
}
