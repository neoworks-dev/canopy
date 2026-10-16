import { describe, expect, test } from "bun:test";
import {
	insideCrumbs,
	isArchiveName,
	joinInside,
	parentInside,
	rowsInside,
	type ArchiveMember,
} from "../ui/lib/archive";

function memberOf(name: string, isDir: boolean, size = 0): ArchiveMember {
	return { name, size, isDir, modTime: "2026-01-01T00:00:00Z", mode: "-rw-r--r--" };
}

const members = [
	memberOf("project", true),
	memberOf("project/top.txt", false, 12),
	memberOf("project/nested/deep.txt", false, 34),
	memberOf("readme.md", false, 56),
];

describe("recognising an archive by name", () => {
	test("the formats the explorer can open are recognised", () => {
		for (const name of ["photos.zip", "PHOTOS.ZIP", "backup.tar", "backup.tar.gz", "b.tbz2"]) {
			expect(isArchiveName(name)).toBe(true);
		}
	});

	test("anything else is an ordinary file", () => {
		for (const name of ["notes.txt", "archive.zip.part", "gzip"]) {
			expect(isArchiveName(name)).toBe(false);
		}
	});
});

describe("one level inside an archive", () => {
	test("the top holds what is not nested under anything", () => {
		const rows = rowsInside(members, "");
		expect(rows.map((row) => row.label)).toEqual(["project", "readme.md"]);
	});

	test("folders come before files", () => {
		const rows = rowsInside([memberOf("a.txt", false), memberOf("b", true)], "");
		expect(rows.map((row) => row.label)).toEqual(["b", "a.txt"]);
	});

	// A tar need not name the folders it holds, only the files inside them.
	test("a folder that only exists in a deeper name still gets a row", () => {
		const rows = rowsInside([memberOf("project/nested/deep.txt", false)], "project");
		expect(rows).toEqual([
			{ label: "nested", name: "project/nested", size: 0, isDir: true, modTime: "" },
		]);
	});

	test("a level carries the full name of what it holds, for descending into", () => {
		const rows = rowsInside(members, "project");
		expect(rows.map((row) => row.name)).toEqual(["project/nested", "project/top.txt"]);
	});

	test("a level nothing sits under is empty rather than showing everything", () => {
		expect(rowsInside(members, "missing")).toEqual([]);
	});
});

describe("moving around inside an archive", () => {
	test("descending appends to the level being looked at", () => {
		expect(joinInside("project", "nested")).toBe("project/nested");
	});

	test("descending from the top needs no separator", () => {
		expect(joinInside("", "project")).toBe("project");
	});

	test("going up drops the last level", () => {
		expect(parentInside("project/nested")).toBe("project");
	});

	test("going up from the first level lands at the top", () => {
		expect(parentInside("project")).toBe("");
	});

	test("the trail across the top carries each level's own path", () => {
		expect(insideCrumbs("project/nested")).toEqual([
			{ label: "project", inside: "project" },
			{ label: "nested", inside: "project/nested" },
		]);
	});

	test("the top of an archive has no trail", () => {
		expect(insideCrumbs("")).toEqual([]);
	});
});
