import { describe, expect, test } from "bun:test";
import {
	sectionsAfterToggle,
	sidebarSectionIds,
	sidebarSectionsFrom,
	type SidebarSectionId,
} from "../ui/lib/sidebar";

describe("folding sidebar groups", () => {
	test("a group that is open folds shut", () => {
		expect(sectionsAfterToggle([], "tags")).toEqual(["tags"]);
	});

	test("a group that is shut folds open again", () => {
		expect(sectionsAfterToggle(["tags", "recent"], "tags")).toEqual(["recent"]);
	});

	test("folding one group leaves the others alone", () => {
		expect(sectionsAfterToggle(["recent"], "devices")).toEqual(["recent", "devices"]);
	});
});

describe("reading the stored fold state", () => {
	test("names this version knows are kept", () => {
		expect(sidebarSectionsFrom(["tags", "recent"])).toEqual(["tags", "recent"]);
	});

	test("a name from another version is dropped rather than folding nothing", () => {
		expect(sidebarSectionsFrom(["tags", "networks"])).toEqual(["tags"]);
	});

	test("a file that says nothing leaves every group open", () => {
		expect(sidebarSectionsFrom(undefined)).toEqual([]);
	});

	test("every group this version renders can be stored", () => {
		expect(sidebarSectionsFrom([...sidebarSectionIds])).toEqual(sidebarSectionIds);
	});
});

describe("the group names", () => {
	test("places is one of them, so the rail always has a group", () => {
		const places: SidebarSectionId = "places";
		expect(sidebarSectionIds).toContain(places);
	});
});
