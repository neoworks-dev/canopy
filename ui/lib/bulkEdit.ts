/**
 * Yazi's bulk create and bulk rename work on a buffer of names, one per line.
 * These read such a buffer back into what should happen to the folder.
 */

export type EntryToCreate = { kind: "file" | "folder"; name: string };

export type RenamePair = { path: string; newName: string };

export type RenamePlan = { pairs: RenamePair[]; errorMessage: string };

/** Blank lines are skipped; a trailing slash makes the line a folder. */
export function entriesToCreate(buffer: string): EntryToCreate[] {
	return linesOf(buffer).flatMap(entryFromLine);
}

function entryFromLine(line: string): EntryToCreate[] {
	const name = line.trim();
	if (name === "" || name === "/") {
		return [];
	}
	if (name.endsWith("/")) {
		return [{ kind: "folder", name: name.slice(0, -1) }];
	}
	return [{ kind: "file", name }];
}

/**
 * Pairs each path with the line at its position. The line count has to match,
 * or a deleted line would shift every later name onto the wrong file.
 */
export function renamePlan(paths: string[], buffer: string): RenamePlan {
	const names = linesOf(buffer.replace(/\n+$/, "")).map((line) => line.trim());
	if (names.length !== paths.length) {
		return { pairs: [], errorMessage: `Expected ${paths.length} names, got ${names.length}.` };
	}
	if (names.some((name) => name === "")) {
		return { pairs: [], errorMessage: "A name cannot be empty." };
	}
	return { pairs: changedPairs(paths, names), errorMessage: "" };
}

function changedPairs(paths: string[], names: string[]): RenamePair[] {
	const pairs: RenamePair[] = [];
	paths.forEach((path, index) => {
		if (baseNameOf(path) !== names[index]) {
			pairs.push({ path, newName: names[index] });
		}
	});
	return pairs;
}

/** The buffer a bulk rename opens with: the current names, one per line. */
export function renameBuffer(paths: string[]): string {
	return paths.map(baseNameOf).join("\n");
}

function linesOf(buffer: string): string[] {
	return buffer.split("\n");
}

function baseNameOf(path: string): string {
	return path.slice(path.lastIndexOf("/") + 1);
}
