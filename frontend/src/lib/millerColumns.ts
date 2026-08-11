/**
 * The column chain a Miller view shows: the folder on screen and every folder
 * on the way to it, each with the child that leads onward.
 *
 * The chain is worked out from the path rather than kept alongside it, so
 * navigating anywhere — a column, the sidebar, the path bar, the history — puts
 * the columns where they belong without anything having to be kept in step.
 */

/** The ancestors of a path, root first, with the path itself last. */
export function ancestorPaths(path: string): string[] {
	if (path === "") {
		return [];
	}
	const names = path.split("/").filter((segment) => segment !== "");
	const paths = ["/"];
	for (const name of names) {
		paths.push(joinOnto(paths[paths.length - 1], name));
	}
	return paths;
}

function joinOnto(parent: string, name: string): string {
	return prefixOf(parent) + name;
}

/**
 * The chain of columns to render for a path. The deepest column is the folder
 * itself, which is the one the pane has already listed.
 */
export function columnChain(path: string, limit: number): string[] {
	const ancestors = ancestorPaths(path);
	if (limit <= 0 || ancestors.length <= limit) {
		return ancestors;
	}
	return ancestors.slice(ancestors.length - limit);
}

/**
 * The entry a column highlights: the child of that column which lies on the way
 * to the folder on screen. The deepest column has none, since nothing leads on
 * from it.
 */
export function leadingChild(columnPath: string, path: string): string {
	if (!isAncestor(columnPath, path)) {
		return "";
	}
	const remainder = path.slice(prefixOf(columnPath).length);
	const next = remainder.split("/")[0];
	if (next === "") {
		return "";
	}
	return prefixOf(columnPath) + next;
}

/** Whether one path contains another, which the root does for everything. */
export function isAncestor(columnPath: string, path: string): boolean {
	if (columnPath === path) {
		return false;
	}
	return path.startsWith(prefixOf(columnPath));
}

// The root already ends in a separator, so joining onto it must not double it.
function prefixOf(columnPath: string): string {
	if (columnPath.endsWith("/")) {
		return columnPath;
	}
	return `${columnPath}/`;
}
