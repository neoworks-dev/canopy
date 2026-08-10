const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/**
 * Human-readable file size using 1024-byte steps. Directories pass a size of 0
 * and render as an em dash instead.
 */
export function formatFileSize(sizeInBytes: number): string {
	if (sizeInBytes < 0) {
		return "—";
	}
	if (sizeInBytes < 1024) {
		return `${sizeInBytes} B`;
	}
	const unitIndex = unitIndexFor(sizeInBytes);
	const scaled = sizeInBytes / Math.pow(1024, unitIndex);
	return `${scaled.toFixed(1)} ${BYTE_UNITS[unitIndex]}`;
}

function unitIndexFor(sizeInBytes: number): number {
	const rawIndex = Math.floor(Math.log(sizeInBytes) / Math.log(1024));
	const maxIndex = BYTE_UNITS.length - 1;
	if (rawIndex > maxIndex) {
		return maxIndex;
	}
	return rawIndex;
}

/** Renders an RFC3339 timestamp from the Go backend as a short local date. */
export function formatModifiedTime(timestamp: string): string {
	if (!timestamp) {
		return "—";
	}
	const parsed = new Date(timestamp);
	if (Number.isNaN(parsed.getTime())) {
		return "—";
	}
	return parsed.toLocaleDateString(undefined, {
		year: "numeric",
		month: "short",
		day: "numeric",
	});
}

/**
 * Splits an absolute path into breadcrumb segments, each carrying the full path
 * needed to navigate back to it.
 */
export function toBreadcrumbs(absolutePath: string): Array<{ label: string; path: string }> {
	const names = absolutePath.split("/").filter((segment) => segment.length > 0);
	const crumbs = [{ label: "/", path: "/" }];
	let accumulated = "";
	for (const name of names) {
		accumulated = `${accumulated}/${name}`;
		crumbs.push({ label: name, path: accumulated });
	}
	return crumbs;
}
