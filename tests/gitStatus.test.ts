import { describe, expect, test } from "bun:test";
import { gitMarkFor } from "../ui/lib/gitStatus";

describe("marking an entry with what git says about it", () => {
	test("an entry git says nothing about carries no mark", () => {
		expect(gitMarkFor(undefined)).toBeNull();
		expect(gitMarkFor("")).toBeNull();
	});

	test("a state from another version is left unmarked rather than mislabelled", () => {
		expect(gitMarkFor("submodule-dirty")).toBeNull();
	});

	test("the states that need attention carry a letter", () => {
		for (const status of ["modified", "added", "deleted", "renamed", "untracked", "conflicted"]) {
			expect(gitMarkFor(status)?.letter).not.toBe("");
		}
	});

	test("an ignored entry is drawn back rather than lettered", () => {
		const mark = gitMarkFor("ignored");

		expect(mark?.isMuted).toBe(true);
		expect(mark?.letter).toBe("");
	});

	test("nothing but ignored entries is drawn back", () => {
		for (const status of ["modified", "added", "deleted", "renamed", "untracked", "conflicted"]) {
			expect(gitMarkFor(status)?.isMuted).toBe(false);
		}
	});
});
