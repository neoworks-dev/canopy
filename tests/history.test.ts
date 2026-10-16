import { describe, expect, test } from "bun:test";
import {
	canGoBack,
	canGoForward,
	currentLocation,
	emptyHistory,
	historyAfterBack,
	historyAfterForward,
	historyAfterVisit,
	type History,
} from "../ui/lib/history";

function visitAll(paths: string[]): History {
	return paths.reduce(historyAfterVisit, emptyHistory);
}

describe("historyAfterVisit", () => {
	test("the first visit becomes the current location", () => {
		const history = historyAfterVisit(emptyHistory, "/home");
		expect(currentLocation(history)).toBe("/home");
		expect(canGoBack(history)).toBe(false);
		expect(canGoForward(history)).toBe(false);
	});

	test("revisiting the current location leaves the stack untouched", () => {
		const history = visitAll(["/home", "/home/photos"]);
		expect(historyAfterVisit(history, "/home/photos")).toBe(history);
	});

	test("a new location after going back drops the forward entries", () => {
		const history = historyAfterBack(visitAll(["/home", "/home/photos", "/home/music"]));
		const branched = historyAfterVisit(history, "/etc");
		expect(branched.entries).toEqual(["/home", "/home/photos", "/etc"]);
		expect(canGoForward(branched)).toBe(false);
	});
});

describe("historyAfterBack and historyAfterForward", () => {
	test("back moves to the previous location and forward returns", () => {
		const history = visitAll(["/home", "/home/photos"]);
		const back = historyAfterBack(history);
		expect(currentLocation(back)).toBe("/home");
		expect(canGoForward(back)).toBe(true);
		expect(currentLocation(historyAfterForward(back))).toBe("/home/photos");
	});

	test("back at the start and forward at the end change nothing", () => {
		const history = visitAll(["/home"]);
		expect(historyAfterBack(history)).toBe(history);
		expect(historyAfterForward(history)).toBe(history);
	});

	test("going back keeps the entries so forward stays available", () => {
		const history = historyAfterBack(visitAll(["/a", "/b", "/c"]));
		expect(history.entries).toEqual(["/a", "/b", "/c"]);
		expect(history.index).toBe(1);
	});
});

describe("currentLocation", () => {
	test("an empty history has no location", () => {
		expect(currentLocation(emptyHistory)).toBe("");
	});
});
