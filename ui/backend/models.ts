// The shapes the Rust commands answer with, one namespace per backend module.

export namespace archive {
	export interface Member {
		name: string;
		size: number;
		isDir: boolean;
		modTime: string;
		mode: string;
	}
}

export namespace audio {
	export interface Details {
		duration: number;
		bitrate: number;
		channels: number;
		sampleRate: number;
		codec: string;
		title: string;
		artist: string;
		album: string;
		waveform: number[];
	}
}

export namespace bookmarks {
	export interface Bookmark {
		name: string;
		path: string;
	}
}

export namespace fileops {
	export interface TrashedItem {
		name: string;
		trashPath: string;
		originalPath: string;
		deletedAt: string;
		size: number;
		isDir: boolean;
	}
}

export namespace filesystem {
	export interface Device {
		name: string;
		path: string;
		isRemovable: boolean;
		totalBytes: number;
		freeBytes: number;
	}
	export interface Entry {
		name: string;
		path: string;
		isDir: boolean;
		size: number;
		modTime: string;
		mode: string;
	}
	export interface Listing {
		path: string;
		parentPath: string;
		entries: Entry[];
	}
	export interface Order {
		key: string;
		descending: boolean;
		directoriesFirst: boolean;
	}
	export interface Place {
		name: string;
		path: string;
		kind: string;
	}
	export interface Preview {
		path: string;
		kind: string;
		text: string;
		tokens: highlight.Token[];
		dataUrl: string;
		truncated: boolean;
		size: number;
		audio: audio.Details;
	}
}

export namespace git {
	export interface Repository {
		root: string;
		branch: string;
	}
	export interface Report {
		repository: Repository;
		isRepo: boolean;
		statuses: Record<string, string>;
	}
}

export namespace highlight {
	export interface Token {
		class: string;
		text: string;
	}
}

export namespace launcher {
	export interface Application {
		id: string;
		name: string;
		icon: string;
		exec: string;
	}
}

export namespace proto {
	export interface Hit {
		path: string;
		name: string;
		isDir: boolean;
		size: number;
		modTimeUnix: number;
	}
	export interface SearchResult {
		hits: Hit[];
		truncated: boolean;
	}
	export interface StatusResult {
		state: string;
		filesIndexed: number;
		scanningPath: string;
		watchedDirectories: number;
		watchLimitReached: boolean;
		lastScanUnix: number;
	}
}

export namespace recents {
	export interface Entry {
		name: string;
		path: string;
		isDir: boolean;
		openedAt: string;
	}
}

export namespace searches {
	export interface Search {
		name: string;
		query: string;
		scope: string;
	}
}

export namespace settings {
	export interface Keybinding {
		key: string;
		command: string;
		when?: string;
	}
	export interface Panes {
		sidebar: number;
		details: number;
	}
	export interface Window {
		width: number;
		height: number;
		x: number;
		y: number;
	}
	export interface Settings {
		version: number;
		viewMode: string;
		columns: string[];
		showHidden: boolean;
		sort: filesystem.Order;
		lastDirectory: string;
		window: Window;
		panes: Panes;
		collapsedSections?: string[];
		showDetails: boolean;
		keybindings?: Keybinding[];
	}
	/** What the UI saves: the store keeps the version, keymap and window geometry itself. */
	export type ViewSettings = Omit<Settings, "version" | "window" | "keybindings">;
}

export namespace thumbnail {
	export interface Thumbnail {
		path: string;
		dataUrl: string;
	}
}

export namespace zoxide {
	export interface Entry {
		path: string;
		score: number;
	}
}
