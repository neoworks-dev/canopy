/**
 * Preview images for the entries a listing draws. The backend owns the shared
 * thumbnail cache on disk; this is the in-memory half, so scrolling back over a
 * folder does not ask for the same image again.
 */
import { ThumbnailFor } from "../backend/FileService";
import { classifyEntry, type FileKind } from "./fileKind";

/** What the backend's renderer can draw. Everything else keeps its icon. */
const renderableKinds = new Set<FileKind>(["image", "video", "pdf"]);

// Data URLs are held in full, so the cache is bounded by count rather than
// left to grow with every picture folder that gets scrolled through.
const maximumCachedThumbnails = 512;

export type ThumbnailSource = {
	name: string;
	path: string;
	isDir: boolean;
	modTime: string;
};

// An entry that turned out to have no preview is cached as an empty string, so
// it is asked about once rather than on every scroll.
const cachedURLs = new Map<string, string>();
const requestsInFlight = new Map<string, Promise<string>>();

export function supportsThumbnail(entry: ThumbnailSource): boolean {
	return renderableKinds.has(classifyEntry(entry));
}

/** What is already known, so a tile that scrolls back into view never flashes. */
export function cachedThumbnail(entry: ThumbnailSource): string {
	const cached = cachedURLs.get(keyFor(entry));
	if (!cached) {
		return "";
	}
	return cached;
}

export async function loadThumbnail(entry: ThumbnailSource): Promise<string> {
	const key = keyFor(entry);
	const cached = cachedURLs.get(key);
	if (cached !== undefined) {
		return cached;
	}
	const inFlight = requestsInFlight.get(key);
	if (inFlight) {
		return inFlight;
	}
	return startRequest(key, entry.path);
}

function startRequest(key: string, path: string): Promise<string> {
	const request = requestThumbnail(path).then((dataURL) => remember(key, dataURL));
	requestsInFlight.set(key, request);
	return request;
}

// A render that fails is a file drawn with its icon, which is what the pane
// shows anyway while the request is in flight.
async function requestThumbnail(path: string): Promise<string> {
	try {
		const thumbnail = await ThumbnailFor(path);
		return thumbnail.dataUrl;
	} catch {
		return "";
	}
}

function remember(key: string, dataURL: string): string {
	requestsInFlight.delete(key);
	cachedURLs.set(key, dataURL);
	evictOldest();
	return dataURL;
}

function evictOldest() {
	while (cachedURLs.size > maximumCachedThumbnails) {
		const oldest = cachedURLs.keys().next().value;
		if (oldest === undefined) {
			return;
		}
		cachedURLs.delete(oldest);
	}
}

// The modification time is part of the key, so an edited file is drawn from a
// fresh render rather than the image of what it used to be.
function keyFor(entry: ThumbnailSource): string {
	return `${entry.path}\n${entry.modTime}`;
}
