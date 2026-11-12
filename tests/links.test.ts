import { describe, expect, test } from "bun:test";
import { linkDestination, normalizedPath } from "../ui/lib/links";

describe("following a link", () => {
	test("a relative target counts from the folder holding the link", () => {
		expect(linkDestination("/home/me/bin/tool", "../src/tool.sh")).toBe("/home/me/src/tool.sh");
		expect(linkDestination("/home/me/latest", "./releases/v2")).toBe("/home/me/releases/v2");
	});

	test("an absolute target is taken as it is, tidied", () => {
		expect(linkDestination("/home/me/x", "/etc//nginx/./sites/")).toBe("/etc/nginx/sites");
	});

	test("climbing past the root stays at the root", () => {
		expect(normalizedPath("/../../tmp")).toBe("/tmp");
		expect(normalizedPath("/")).toBe("/");
	});
});
