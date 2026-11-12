/** Where a symlink leads, read from its target as written beside it. */
import { parentDirectoryOf } from "./format";

/** A relative target counts from the folder holding the link, as the kernel reads it. */
export function linkDestination(linkPath: string, target: string): string {
	if (target.startsWith("/")) {
		return normalizedPath(target);
	}
	return normalizedPath(`${parentDirectoryOf(linkPath)}/${target}`);
}

/** Folds `.` and `..` away; `..` at the root stays at the root. */
export function normalizedPath(path: string): string {
	const kept: string[] = [];
	for (const segment of path.split("/")) {
		applySegment(kept, segment);
	}
	return `/${kept.join("/")}`;
}

function applySegment(kept: string[], segment: string) {
	if (segment === "" || segment === ".") {
		return;
	}
	if (segment === "..") {
		kept.pop();
		return;
	}
	kept.push(segment);
}
