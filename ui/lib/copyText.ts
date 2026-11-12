/**
 * The text Yazi's `c …` keys put on the clipboard: one line per selected entry,
 * as a path, a file URL, the folder holding it, or just its name.
 */
import { parentDirectoryOf } from "./format";

export type CopyTextKind = "path" | "url" | "dirpath" | "dirurl" | "filename" | "nameWithoutExtension";

export function copyTextFor(paths: string[], kind: CopyTextKind): string {
	return paths.map((path) => lineFor(path, kind)).join("\n");
}

function lineFor(path: string, kind: CopyTextKind): string {
	switch (kind) {
		case "path":
			return path;
		case "url":
			return fileUrlOf(path);
		case "dirpath":
			return parentDirectoryOf(path);
		case "dirurl":
			return fileUrlOf(parentDirectoryOf(path));
		case "filename":
			return baseNameOf(path);
		case "nameWithoutExtension":
			return withoutExtension(baseNameOf(path));
	}
}

/** Each segment is percent-encoded, so the separators survive and spaces do not. */
export function fileUrlOf(path: string): string {
	const segments = path.split("/").map((segment) => encodeURIComponent(segment));
	return `file://${segments.join("/")}`;
}

function baseNameOf(path: string): string {
	const segments = path.split("/").filter(Boolean);
	if (segments.length === 0) {
		return "/";
	}
	return segments[segments.length - 1];
}

/** A leading dot names a hidden file rather than starting an extension. */
export function withoutExtension(name: string): string {
	const dot = name.lastIndexOf(".");
	if (dot <= 0) {
		return name;
	}
	return name.slice(0, dot);
}
