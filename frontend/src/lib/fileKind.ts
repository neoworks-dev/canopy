export type FileKind =
	| "folder"
	| "image"
	| "audio"
	| "video"
	| "archive"
	| "pdf"
	| "code"
	| "text"
	| "file";

const EXTENSIONS_BY_KIND: Record<string, FileKind> = {
	png: "image",
	jpg: "image",
	jpeg: "image",
	gif: "image",
	webp: "image",
	svg: "image",
	avif: "image",
	bmp: "image",
	ico: "image",

	mp3: "audio",
	flac: "audio",
	wav: "audio",
	ogg: "audio",
	m4a: "audio",
	opus: "audio",

	mp4: "video",
	mkv: "video",
	webm: "video",
	mov: "video",
	avi: "video",

	zip: "archive",
	tar: "archive",
	gz: "archive",
	xz: "archive",
	zst: "archive",
	bz2: "archive",
	rar: "archive",
	"7z": "archive",

	pdf: "pdf",

	go: "code",
	ts: "code",
	js: "code",
	svelte: "code",
	rs: "code",
	py: "code",
	c: "code",
	h: "code",
	cpp: "code",
	css: "code",
	html: "code",
	json: "code",
	toml: "code",
	yaml: "code",
	yml: "code",
	sh: "code",

	md: "text",
	txt: "text",
	log: "text",
	csv: "text",
};

export function classifyEntry(entry: { name: string; isDir: boolean }): FileKind {
	if (entry.isDir) {
		return "folder";
	}
	const extension = extensionOf(entry.name);
	const kind = EXTENSIONS_BY_KIND[extension];
	if (!kind) {
		return "file";
	}
	return kind;
}

function extensionOf(name: string): string {
	const lastDot = name.lastIndexOf(".");
	if (lastDot <= 0) {
		return "";
	}
	return name.slice(lastDot + 1).toLowerCase();
}
