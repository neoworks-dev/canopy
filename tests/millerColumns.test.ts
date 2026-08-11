import { describe, expect, test } from "bun:test";
import {
	ancestorPaths,
	columnChain,
	isAncestor,
	leadingChild,
} from "../frontend/src/lib/millerColumns";

describe("the folders on the way to a path", () => {
	test("a nested path is every folder above it, root first", () => {
		expect(ancestorPaths("/home/moritz/notes")).toEqual([
			"/",
			"/home",
			"/home/moritz",
			"/home/moritz/notes",
		]);
	});

	test("the root is the only folder above itself", () => {
		expect(ancestorPaths("/")).toEqual(["/"]);
	});

	test("a trailing separator does not add an unnamed folder", () => {
		expect(ancestorPaths("/home/moritz/")).toEqual(["/", "/home", "/home/moritz"]);
	});

	test("no path has no folders", () => {
		expect(ancestorPaths("")).toEqual([]);
	});
});

describe("the columns a pane has room for", () => {
	test("a shallow path shows every folder above it", () => {
		expect(columnChain("/home/moritz", 4)).toEqual(["/", "/home", "/home/moritz"]);
	});

	test("a deep path keeps the folders nearest the one on screen", () => {
		expect(columnChain("/a/b/c/d/e", 3)).toEqual(["/a/b/c", "/a/b/c/d", "/a/b/c/d/e"]);
	});

	test("the folder on screen is always the last column", () => {
		const chain = columnChain("/a/b/c/d/e", 2);
		expect(chain[chain.length - 1]).toBe("/a/b/c/d/e");
	});

	test("no limit means no truncation", () => {
		expect(columnChain("/a/b", 0)).toEqual(["/", "/a", "/a/b"]);
	});
});

describe("the entry a column marks", () => {
	test("a column marks the child that leads onward", () => {
		expect(leadingChild("/home", "/home/moritz/notes")).toBe("/home/moritz");
	});

	test("the root marks its child without doubling the separator", () => {
		expect(leadingChild("/", "/home/moritz")).toBe("/home");
	});

	test("the folder on screen leads nowhere", () => {
		expect(leadingChild("/home/moritz", "/home/moritz")).toBe("");
	});

	test("a folder off the chain marks nothing", () => {
		expect(leadingChild("/etc", "/home/moritz")).toBe("");
	});

	test("a folder whose name is a prefix of another is not on the chain", () => {
		expect(leadingChild("/home/mo", "/home/moritz/notes")).toBe("");
	});
});

describe("whether one folder contains another", () => {
	test("a parent contains its child", () => {
		expect(isAncestor("/home", "/home/moritz")).toBe(true);
	});

	test("the root contains everything", () => {
		expect(isAncestor("/", "/home")).toBe(true);
	});

	test("a folder does not contain itself", () => {
		expect(isAncestor("/home", "/home")).toBe(false);
	});

	test("a matching prefix is not containment", () => {
		expect(isAncestor("/home/mo", "/home/moritz")).toBe(false);
	});
});
