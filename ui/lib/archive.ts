/**
 * Looking inside an archive. The Go side reports every member it holds as one
 * flat list; a pane shows one folder at a time, which is what folding that list
 * into a level of its own is for.
 */

/** One entry inside an archive, named by the path it holds inside. */
export type ArchiveMember = {
	name: string;
	size: number;
	isDir: boolean;
	modTime: string;
	mode: string;
};

/** A member as one line of a listing, at the level being looked at. */
export type ArchiveRow = {
	/** The last part of the name, which is what the row is labelled with. */
	label: string;
	/** The full path inside the archive, which is where opening it leads. */
	name: string;
	size: number;
	isDir: boolean;
	modTime: string;
};

// The suffixes the Go side knows how to open, longest first so a .tar.gz is not
// read as the .gz it also ends in.
const archiveSuffixes = [".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar", ".zip"];

export function isArchiveName(name: string): boolean {
	const lowered = name.toLowerCase();
	return archiveSuffixes.some((suffix) => lowered.endsWith(suffix));
}

/**
 * The rows one level inside an archive holds. A folder that only exists as part
 * of a deeper member's name still gets a row: a tar need not name its folders,
 * and a level with nothing in it is not what the archive holds.
 */
export function rowsInside(members: ArchiveMember[], inside: string): ArchiveRow[] {
	const rows = new Map<string, ArchiveRow>();
	for (const member of members) {
		addRowFor(rows, member, inside);
	}
	return sortRows([...rows.values()]);
}

function addRowFor(rows: Map<string, ArchiveRow>, member: ArchiveMember, inside: string) {
	const remainder = remainderOf(member.name, inside);
	if (!remainder) {
		return;
	}
	const separator = remainder.indexOf("/");
	if (separator < 0) {
		rows.set(remainder, rowOf(member, remainder));
		return;
	}
	rememberFolder(rows, remainder.slice(0, separator), inside);
}

function rowOf(member: ArchiveMember, label: string): ArchiveRow {
	return {
		label,
		name: member.name,
		size: member.size,
		isDir: member.isDir,
		modTime: member.modTime,
	};
}

// A folder implied by a deeper member has no member of its own to describe it.
function rememberFolder(rows: Map<string, ArchiveRow>, label: string, inside: string) {
	if (rows.has(label)) {
		return;
	}
	rows.set(label, { label, name: joinInside(inside, label), size: 0, isDir: true, modTime: "" });
}

/** What is left of a member's name below the level being looked at. */
function remainderOf(name: string, inside: string): string {
	if (!inside) {
		return name;
	}
	if (!name.startsWith(`${inside}/`)) {
		return "";
	}
	return name.slice(inside.length + 1);
}

export function joinInside(inside: string, label: string): string {
	if (!inside) {
		return label;
	}
	return `${inside}/${label}`;
}

/** The level above the one being looked at, empty at the top of the archive. */
export function parentInside(inside: string): string {
	const separator = inside.lastIndexOf("/");
	if (separator < 0) {
		return "";
	}
	return inside.slice(0, separator);
}

/** The levels down to the one being looked at, for the trail across the top. */
export function insideCrumbs(inside: string): Array<{ label: string; inside: string }> {
	const crumbs: Array<{ label: string; inside: string }> = [];
	let accumulated = "";
	for (const label of inside.split("/").filter(Boolean)) {
		accumulated = joinInside(accumulated, label);
		crumbs.push({ label, inside: accumulated });
	}
	return crumbs;
}

// Folders first and then by name, which is how the listing beside it is ordered.
function sortRows(rows: ArchiveRow[]): ArchiveRow[] {
	return rows.sort((first, second) => {
		if (first.isDir !== second.isDir) {
			return Number(second.isDir) - Number(first.isDir);
		}
		return first.label.localeCompare(second.label);
	});
}
