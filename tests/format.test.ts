import { describe, expect, test } from "bun:test";
import {
	formatFileSize,
	formatModifiedTime,
	formatModifiedUnixTime,
	parentDirectoryOf,
	toBreadcrumbs,
} from "../frontend/src/lib/format";

describe("formatFileSize", () => {
	test("reports raw bytes below one kilobyte", () => {
		expect(formatFileSize(0)).toBe("0 B");
		expect(formatFileSize(512)).toBe("512 B");
		expect(formatFileSize(1023)).toBe("1023 B");
	});

	test("steps up in 1024-byte units", () => {
		expect(formatFileSize(1024)).toBe("1.0 KB");
		expect(formatFileSize(1536)).toBe("1.5 KB");
		expect(formatFileSize(1024 * 1024)).toBe("1.0 MB");
		expect(formatFileSize(1024 * 1024 * 1024)).toBe("1.0 GB");
	});

	test("clamps to the largest known unit", () => {
		const beyondPetabytes = Math.pow(1024, 8);
		expect(formatFileSize(beyondPetabytes)).toEndWith(" PB");
	});

	test("renders negative sizes as an em dash", () => {
		expect(formatFileSize(-1)).toBe("—");
	});
});

describe("formatModifiedTime", () => {
	test("formats an RFC3339 timestamp", () => {
		expect(formatModifiedTime("2026-08-10T12:30:00Z")).not.toBe("—");
	});

	test("falls back to an em dash for empty or invalid input", () => {
		expect(formatModifiedTime("")).toBe("—");
		expect(formatModifiedTime("not-a-date")).toBe("—");
	});
});

describe("formatModifiedUnixTime", () => {
	test("formats seconds since the epoch", () => {
		const sameInstant = Date.parse("2026-08-10T12:30:00Z") / 1000;
		expect(formatModifiedUnixTime(sameInstant)).toBe(
			formatModifiedTime("2026-08-10T12:30:00Z"),
		);
	});

	test("renders a missing timestamp as an em dash", () => {
		expect(formatModifiedUnixTime(0)).toBe("—");
		expect(formatModifiedUnixTime(-1)).toBe("—");
	});
});

describe("parentDirectoryOf", () => {
	test("returns the containing directory", () => {
		expect(parentDirectoryOf("/home/moritz/notes.txt")).toBe("/home/moritz");
		expect(parentDirectoryOf("/home/moritz/Documents")).toBe("/home/moritz");
	});

	test("stops at the filesystem root", () => {
		expect(parentDirectoryOf("/home")).toBe("/");
		expect(parentDirectoryOf("/")).toBe("/");
	});
});

describe("toBreadcrumbs", () => {
	test("always starts at the filesystem root", () => {
		expect(toBreadcrumbs("/")).toEqual([{ label: "/", path: "/" }]);
	});

	test("accumulates the full path for each segment", () => {
		expect(toBreadcrumbs("/home/moritz/Documents")).toEqual([
			{ label: "/", path: "/" },
			{ label: "home", path: "/home" },
			{ label: "moritz", path: "/home/moritz" },
			{ label: "Documents", path: "/home/moritz/Documents" },
		]);
	});

	test("ignores a trailing slash", () => {
		expect(toBreadcrumbs("/home/moritz/")).toEqual([
			{ label: "/", path: "/" },
			{ label: "home", path: "/home" },
			{ label: "moritz", path: "/home/moritz" },
		]);
	});
});
