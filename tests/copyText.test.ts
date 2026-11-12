import { describe, expect, test } from "bun:test";
import { copyTextFor, fileUrlOf, withoutExtension } from "../ui/lib/copyText";

describe("the text Yazi's copy keys produce", () => {
	const paths = ["/home/me/notes.txt", "/home/me/My Photos/beach.tar.gz"];

	test("paths and names go one per line", () => {
		expect(copyTextFor(paths, "path")).toBe("/home/me/notes.txt\n/home/me/My Photos/beach.tar.gz");
		expect(copyTextFor(paths, "filename")).toBe("notes.txt\nbeach.tar.gz");
	});

	test("the folder holding each entry", () => {
		expect(copyTextFor(paths, "dirpath")).toBe("/home/me\n/home/me/My Photos");
	});

	test("only the last extension is dropped", () => {
		expect(copyTextFor(paths, "nameWithoutExtension")).toBe("notes\nbeach.tar");
	});

	test("a hidden file's leading dot is not an extension", () => {
		expect(withoutExtension(".bashrc")).toBe(".bashrc");
		expect(withoutExtension(".config.json")).toBe(".config");
	});

	test("URLs percent-encode each segment and keep the separators", () => {
		expect(fileUrlOf("/home/me/My Photos/a#b.png")).toBe("file:///home/me/My%20Photos/a%23b.png");
		expect(copyTextFor(["/tmp/x"], "dirurl")).toBe("file:///tmp");
	});
});
