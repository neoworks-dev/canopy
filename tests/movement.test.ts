import { describe, expect, test } from "bun:test";
import { targetIndexFor, type MovementContext } from "../frontend/src/lib/movement";

const rows: MovementContext = { total: 10, columns: 1, pageSize: 4 };
const grid: MovementContext = { total: 10, columns: 4, pageSize: 8 };

describe("targetIndexFor in the row view", () => {
	test("down and up step by one", () => {
		expect(targetIndexFor("ArrowDown", 3, rows)).toBe(4);
		expect(targetIndexFor("ArrowUp", 3, rows)).toBe(2);
	});

	test("left and right do nothing where a row holds one entry", () => {
		expect(targetIndexFor("ArrowLeft", 3, rows)).toBe(-1);
		expect(targetIndexFor("ArrowRight", 3, rows)).toBe(-1);
	});

	test("movement stops at the ends instead of wrapping", () => {
		expect(targetIndexFor("ArrowUp", 0, rows)).toBe(0);
		expect(targetIndexFor("ArrowDown", 9, rows)).toBe(9);
	});
});

describe("targetIndexFor in the grid view", () => {
	test("down and up step by the number of rendered columns", () => {
		expect(targetIndexFor("ArrowDown", 1, grid)).toBe(5);
		expect(targetIndexFor("ArrowUp", 6, grid)).toBe(2);
	});

	test("left and right step within the row", () => {
		expect(targetIndexFor("ArrowRight", 1, grid)).toBe(2);
		expect(targetIndexFor("ArrowLeft", 1, grid)).toBe(0);
	});

	test("a partial last row clamps to the last entry", () => {
		expect(targetIndexFor("ArrowDown", 7, grid)).toBe(9);
	});
});

describe("targetIndexFor edges and pages", () => {
	test("Home and End jump to the first and last entry", () => {
		expect(targetIndexFor("Home", 5, rows)).toBe(0);
		expect(targetIndexFor("End", 5, rows)).toBe(9);
	});

	test("PageDown and PageUp move by one viewport", () => {
		expect(targetIndexFor("PageDown", 1, rows)).toBe(5);
		expect(targetIndexFor("PageUp", 8, rows)).toBe(4);
	});

	test("the first keypress lands on the first entry", () => {
		expect(targetIndexFor("ArrowDown", -1, rows)).toBe(0);
		expect(targetIndexFor("ArrowUp", -1, grid)).toBe(0);
	});

	test("an unrelated key is not movement", () => {
		expect(targetIndexFor("Enter", 3, rows)).toBe(-1);
	});

	test("an empty listing has nowhere to move", () => {
		expect(targetIndexFor("ArrowDown", -1, { total: 0, columns: 1, pageSize: 4 })).toBe(-1);
	});
});
