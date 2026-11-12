import { describe, expect, test } from "bun:test";
import { entriesToCreate, renameBuffer, renamePlan } from "../ui/lib/bulkEdit";

describe("bulk create", () => {
	test("a trailing slash makes a folder, blank lines are skipped", () => {
		expect(entriesToCreate("notes.md\n\n  src/ \n/\nREADME")).toEqual([
			{ kind: "file", name: "notes.md" },
			{ kind: "folder", name: "src" },
			{ kind: "file", name: "README" },
		]);
	});
});

describe("bulk rename", () => {
	const paths = ["/w/a.txt", "/w/b.txt", "/w/c.txt"];

	test("the buffer opens with the current names", () => {
		expect(renameBuffer(paths)).toBe("a.txt\nb.txt\nc.txt");
	});

	test("only the lines that changed are renamed", () => {
		expect(renamePlan(paths, "a.txt\nbee.txt\nc.md\n")).toEqual({
			pairs: [
				{ path: "/w/b.txt", newName: "bee.txt" },
				{ path: "/w/c.txt", newName: "c.md" },
			],
			errorMessage: "",
		});
	});

	test("a missing line is refused rather than shifting names onto other files", () => {
		const plan = renamePlan(paths, "a.txt\nc.txt");
		expect(plan.pairs).toEqual([]);
		expect(plan.errorMessage).toBe("Expected 3 names, got 2.");
	});

	test("an emptied line is refused", () => {
		expect(renamePlan(paths, "a.txt\n\nc.txt").errorMessage).toBe("A name cannot be empty.");
	});
});
