import { describe, expect, test } from "bun:test";
import { summarize } from "../ui/lib/statusStrip";

function file(name: string, size: number) {
	return {
		name,
		path: `/home/moritz/${name}`,
		isDir: false,
		size,
		modTime: "2026-03-09T10:00:00Z",
		mode: "-rw-r--r--",
	};
}

function folder(name: string) {
	return { ...file(name, 4096), isDir: true };
}

describe("with nothing selected", () => {
	test("the folder's own count is what the strip carries", () => {
		expect(summarize({ entryCount: 42, hiddenCount: 0, selected: [] })).toEqual({
			primary: "42 items",
			secondary: "",
		});
	});

	test("one entry is not called one items", () => {
		expect(summarize({ entryCount: 1, hiddenCount: 0, selected: [] }).primary).toBe("1 item");
	});

	test("entries kept off screen are counted rather than left unexplained", () => {
		expect(summarize({ entryCount: 3, hiddenCount: 8, selected: [] }).secondary).toBe("8 hidden");
	});
});

describe("with one entry selected", () => {
	test("its name leads and its details follow", () => {
		const summary = summarize({ entryCount: 9, hiddenCount: 0, selected: [file("report.pdf", 2048)] });

		expect(summary.primary).toBe("report.pdf");
		expect(summary.secondary).toContain("PDF");
		expect(summary.secondary).toContain("2.0 KB");
	});

	test("a folder reports no size, since it does not know its own", () => {
		const summary = summarize({ entryCount: 9, hiddenCount: 0, selected: [folder("Projects")] });

		expect(summary.primary).toBe("Projects");
		expect(summary.secondary).toContain("Folder");
		expect(summary.secondary).not.toContain("KB");
	});
});

describe("with several entries selected", () => {
	test("the count is set against the folder's own", () => {
		const summary = summarize({
			entryCount: 42,
			hiddenCount: 0,
			selected: [file("a.txt", 1024), file("b.txt", 1024)],
		});

		expect(summary.primary).toBe("2 of 42 items selected");
		expect(summary.secondary).toBe("2.0 KB");
	});

	test("folders in the selection add nothing to the total", () => {
		const summary = summarize({
			entryCount: 42,
			hiddenCount: 0,
			selected: [folder("Projects"), file("a.txt", 1024)],
		});

		expect(summary.secondary).toBe("1.0 KB");
	});
});
