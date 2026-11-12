import { describe, expect, test } from "bun:test";
import type { filesystem } from "../ui/backend/models";
import { spotLines } from "../ui/lib/spot";

function entry(overrides: Partial<filesystem.Entry>): filesystem.Entry {
	return {
		name: "notes.txt",
		path: "/home/me/notes.txt",
		isDir: false,
		size: 2048,
		modTime: "2026-03-09T10:00:00Z",
		mode: "-rw-r--r--",
		createdTime: "",
		owner: "me:users",
		isSymlink: false,
		linkTarget: "",
		...overrides,
	};
}

function labels(lines: { label: string }[]): string[] {
	return lines.map((line) => line.label);
}

describe("the spot", () => {
	test("a file shows its exact size and its owner", () => {
		const lines = spotLines(entry({}));
		expect(lines.find((line) => line.label === "Size")?.value).toBe("2.0 KB (2,048 bytes)");
		expect(lines.find((line) => line.label === "Owner")?.value).toBe("me:users");
	});

	test("a folder has no size line, and only a link says where it points", () => {
		expect(labels(spotLines(entry({ isDir: true })))).not.toContain("Size");
		expect(labels(spotLines(entry({})))).not.toContain("Links to");
		const link = spotLines(entry({ isSymlink: true, linkTarget: "../real.txt" }));
		expect(link.find((line) => line.label === "Links to")?.value).toBe("../real.txt");
	});
});
