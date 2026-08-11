import { beforeEach, expect, mock, test } from "bun:test";

let requestedPaths: string[] = [];

mock.module("../frontend/wailsjs/go/main/FileService", () => ({
	ThumbnailFor: async (path: string) => {
		requestedPaths.push(path);
		return { path, dataUrl: `data:image/png;base64,${path}` };
	},
}));

const { cachedThumbnail, loadThumbnail, supportsThumbnail } = await import(
	"../frontend/src/lib/thumbnails"
);

function entry(name: string, modTime = "2026-01-01") {
	return { name, path: `/photos/${name}`, isDir: false, modTime };
}

beforeEach(() => {
	requestedPaths = [];
});

test("only the kinds the renderer can draw are asked about", () => {
	expect(supportsThumbnail(entry("holiday.jpg"))).toBe(true);
	expect(supportsThumbnail(entry("clip.mkv"))).toBe(true);
	expect(supportsThumbnail(entry("paper.pdf"))).toBe(true);
	expect(supportsThumbnail(entry("notes.txt"))).toBe(false);
	expect(supportsThumbnail({ ...entry("Pictures"), isDir: true })).toBe(false);
});

test("a preview is requested once and served from memory after that", async () => {
	const first = await loadThumbnail(entry("a.jpg"));
	const second = await loadThumbnail(entry("a.jpg"));

	expect(second).toBe(first);
	expect(requestedPaths).toEqual(["/photos/a.jpg"]);
});

test("requests for the same entry in flight together share one render", async () => {
	const [first, second] = await Promise.all([
		loadThumbnail(entry("b.jpg")),
		loadThumbnail(entry("b.jpg")),
	]);

	expect(second).toBe(first);
	expect(requestedPaths).toEqual(["/photos/b.jpg"]);
});

test("an edited file is rendered again rather than drawn from the old image", async () => {
	await loadThumbnail(entry("c.jpg", "2026-01-01"));
	await loadThumbnail(entry("c.jpg", "2026-06-01"));

	expect(requestedPaths).toEqual(["/photos/c.jpg", "/photos/c.jpg"]);
});

test("what is already cached is readable without waiting", async () => {
	expect(cachedThumbnail(entry("d.jpg"))).toBe("");

	await loadThumbnail(entry("d.jpg"));

	expect(cachedThumbnail(entry("d.jpg"))).toBe("data:image/png;base64,/photos/d.jpg");
});
