import { describe, expect, test } from "bun:test";
import { classifyEntry } from "../ui/lib/fileKind";

describe("classifyEntry", () => {
	test("directories win over any extension in their name", () => {
		expect(classifyEntry({ name: "photos.zip", isDir: true })).toBe("folder");
	});

	test("maps known extensions to their kind", () => {
		expect(classifyEntry({ name: "cover.PNG", isDir: false })).toBe("image");
		expect(classifyEntry({ name: "track.flac", isDir: false })).toBe("audio");
		expect(classifyEntry({ name: "clip.mkv", isDir: false })).toBe("video");
		expect(classifyEntry({ name: "backup.tar", isDir: false })).toBe("archive");
		expect(classifyEntry({ name: "manual.pdf", isDir: false })).toBe("pdf");
		expect(classifyEntry({ name: "main.go", isDir: false })).toBe("code");
		expect(classifyEntry({ name: "notes.md", isDir: false })).toBe("text");
	});

	test("falls back to a generic file for unknown extensions", () => {
		expect(classifyEntry({ name: "archive.qqq", isDir: false })).toBe("file");
		expect(classifyEntry({ name: "LICENSE", isDir: false })).toBe("file");
	});

	test("treats a dotfile without a second dot as extensionless", () => {
		expect(classifyEntry({ name: ".bashrc", isDir: false })).toBe("file");
	});

	test("uses the last extension of a multi-part name", () => {
		expect(classifyEntry({ name: "site.tar.gz", isDir: false })).toBe("archive");
	});
});
