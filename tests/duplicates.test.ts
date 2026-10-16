import { describe, expect, test } from "bun:test";
import {
	extraPathsOf,
	groupLabel,
	recoverableLabel,
	remainingGroups,
	searchProgressLabel,
	type DuplicateReport,
} from "../ui/lib/duplicates";

function reportOf(fields: Partial<DuplicateReport>): DuplicateReport {
	return {
		id: "duplicates-1",
		root: "/home/moritz/Downloads",
		stage: "walking",
		scanned: 0,
		candidates: 0,
		compared: 0,
		unreadable: 0,
		recoverableBytes: 0,
		groups: [],
		cancelled: false,
		...fields,
	};
}

describe("what the search says it is doing", () => {
	test("a search that has not reported yet says it is starting", () => {
		expect(searchProgressLabel(null)).toBe("Starting…");
	});

	test("the walk counts the files it has seen", () => {
		const report = reportOf({ stage: "walking", scanned: 12043 });
		expect(searchProgressLabel(report)).toBe("Looking through 12,043 files");
	});

	test("the comparison counts against what is worth reading, not the whole tree", () => {
		const report = reportOf({ stage: "comparing", scanned: 12043, compared: 84, candidates: 312 });
		expect(searchProgressLabel(report)).toBe("Comparing 84 of 312 files");
	});

	test("finding nothing is an answer rather than an empty pane", () => {
		const report = reportOf({ stage: "done", scanned: 900 });
		expect(searchProgressLabel(report)).toBe("No duplicates among 900 files");
	});

	test("a search that was stopped does not claim to have covered the folder", () => {
		const report = reportOf({ stage: "done", scanned: 900, cancelled: true });
		expect(searchProgressLabel(report)).toBe("Stopped after 900 files");
	});
});

describe("what a set of copies says", () => {
	test("a group reports how many copies there are and what each one costs", () => {
		expect(groupLabel({ size: 2048, paths: ["/a", "/b", "/c"] })).toBe("3 copies, 2.0 KB each");
	});

	test("the space to be had is what deleting all but one of each would free", () => {
		expect(recoverableLabel(reportOf({ recoverableBytes: 4096 }))).toBe("4.0 KB can be freed");
	});

	test("trashing the rest of a group keeps the first copy", () => {
		expect(extraPathsOf({ size: 10, paths: ["/a", "/b", "/c"] })).toEqual(["/b", "/c"]);
	});
});

describe("what is left after trashing", () => {
	const groups = [
		{ size: 10, paths: ["/a", "/b", "/c"] },
		{ size: 20, paths: ["/d", "/e"] },
	];

	test("a trashed copy leaves the group it was in", () => {
		expect(remainingGroups(groups, new Set(["/b"]))).toEqual([
			{ size: 10, paths: ["/a", "/c"] },
			{ size: 20, paths: ["/d", "/e"] },
		]);
	});

	test("a group down to one file is no longer a set of copies", () => {
		expect(remainingGroups(groups, new Set(["/d"]))).toEqual([
			{ size: 10, paths: ["/a", "/b", "/c"] },
		]);
	});

	test("nothing trashed leaves every group as it was", () => {
		expect(remainingGroups(groups, new Set())).toEqual(groups);
	});
});
