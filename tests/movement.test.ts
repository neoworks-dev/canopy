import { describe, expect, test } from "bun:test";
import { targetIndexFor, type MovementContext } from "../frontend/src/lib/movement";

const rows: MovementContext = { total: 10, columns: 1, pageSize: 4 };
const grid: MovementContext = { total: 10, columns: 4, pageSize: 8 };

describe("targetIndexFor in the row view", () => {
	test("down and up step by one", () => {
		expect(targetIndexFor("down", 3, rows)).toBe(4);
		expect(targetIndexFor("up", 3, rows)).toBe(2);
	});

	test("left and right do nothing where a row holds one entry", () => {
		expect(targetIndexFor("left", 3, rows)).toBe(-1);
		expect(targetIndexFor("right", 3, rows)).toBe(-1);
	});

	test("movement stops at the ends instead of wrapping", () => {
		expect(targetIndexFor("up", 0, rows)).toBe(0);
		expect(targetIndexFor("down", 9, rows)).toBe(9);
	});
});

describe("targetIndexFor in the grid view", () => {
	test("down and up step by the number of rendered columns", () => {
		expect(targetIndexFor("down", 1, grid)).toBe(5);
		expect(targetIndexFor("up", 6, grid)).toBe(2);
	});

	test("left and right step within the row", () => {
		expect(targetIndexFor("right", 1, grid)).toBe(2);
		expect(targetIndexFor("left", 1, grid)).toBe(0);
	});

	test("a partial last row clamps to the last entry", () => {
		expect(targetIndexFor("down", 7, grid)).toBe(9);
	});
});

describe("targetIndexFor edges and pages", () => {
	test("first and last jump to the ends of the listing", () => {
		expect(targetIndexFor("first", 5, rows)).toBe(0);
		expect(targetIndexFor("last", 5, rows)).toBe(9);
	});

	test("page movements cover one viewport", () => {
		expect(targetIndexFor("pageDown", 1, rows)).toBe(5);
		expect(targetIndexFor("pageUp", 8, rows)).toBe(4);
	});

	test("the first movement lands on the first entry", () => {
		expect(targetIndexFor("down", -1, rows)).toBe(0);
		expect(targetIndexFor("up", -1, grid)).toBe(0);
	});

	test("an empty listing has nowhere to move", () => {
		expect(targetIndexFor("down", -1, { total: 0, columns: 1, pageSize: 4 })).toBe(-1);
	});
});
