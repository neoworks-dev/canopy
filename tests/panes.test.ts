import { describe, expect, test } from "bun:test";
import {
	clampPaneWidth,
	defaultPaneWidths,
	minimumPaneWidth,
	paneWidthAfterDrag,
	paneWidthFrom,
} from "../ui/lib/panes";

describe("clampPaneWidth", () => {
	test("leaves a width between the limits alone", () => {
		expect(clampPaneWidth("left", 240)).toBe(240);
	});

	test("holds the minimum", () => {
		expect(clampPaneWidth("left", 20)).toBe(minimumPaneWidth);
		expect(clampPaneWidth("right", 20)).toBe(minimumPaneWidth);
	});

	test("rounds to whole pixels", () => {
		expect(clampPaneWidth("left", 240.4)).toBe(240);
		expect(clampPaneWidth("right", 240.5)).toBe(241);
	});

	test("caps each side at its own maximum", () => {
		expect(clampPaneWidth("left", 5000)).toBe(420);
		expect(clampPaneWidth("right", 5000)).toBe(640);
	});
});

describe("paneWidthAfterDrag", () => {
	test("a rightward drag widens the pane on the left of the gutter", () => {
		expect(paneWidthAfterDrag("left", 208, 40)).toBe(248);
	});

	test("a rightward drag narrows the pane on the right of the gutter", () => {
		expect(paneWidthAfterDrag("right", 320, 40)).toBe(280);
	});

	test("a fractional drag still gives a whole width", () => {
		expect(Number.isInteger(paneWidthAfterDrag("left", 208, 12.37))).toBe(true);
		expect(Number.isInteger(paneWidthAfterDrag("right", 320, 12.37))).toBe(true);
	});

	test("a drag past the limits stops at them", () => {
		expect(paneWidthAfterDrag("left", 208, -1000)).toBe(minimumPaneWidth);
		expect(paneWidthAfterDrag("right", 320, -1000)).toBe(640);
	});
});

describe("paneWidthFrom", () => {
	test("keeps a stored width", () => {
		expect(paneWidthFrom("left", 300, defaultPaneWidths.sidebar)).toBe(300);
	});

	test("clamps a stored width that is out of range", () => {
		expect(paneWidthFrom("left", 900, defaultPaneWidths.sidebar)).toBe(420);
	});

	test("falls back when the file said nothing usable", () => {
		expect(paneWidthFrom("left", undefined, defaultPaneWidths.sidebar)).toBe(208);
		expect(paneWidthFrom("right", "320", defaultPaneWidths.details)).toBe(320);
		expect(paneWidthFrom("right", 0, defaultPaneWidths.details)).toBe(320);
		expect(paneWidthFrom("right", Number.NaN, defaultPaneWidths.details)).toBe(320);
	});
});
