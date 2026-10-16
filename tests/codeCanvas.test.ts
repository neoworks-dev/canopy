import { expect, test } from "bun:test";

import {
	layoutLines,
	longestLine,
	usedClasses,
	visibleLines,
} from "../ui/lib/codeCanvas";

function token(text: string, className = "") {
	return { class: className, text } as never;
}

test("a token spanning newlines is split across lines", () => {
	const lines = layoutLines([token("one\ntwo\nthree")]);

	expect(lines).toHaveLength(3);
	expect(lines[1][0].text).toBe("two");
});

test("runs on one line follow each other by column", () => {
	const lines = layoutLines([token("const", "hl-k"), token(" "), token("value", "hl-nf")]);

	expect(lines[0].map((run) => run.column)).toEqual([0, 5, 6]);
	expect(lines[0][2].className).toBe("hl-nf");
});

test("a tab reaches the next stop rather than counting as one column", () => {
	const lines = layoutLines([token("ab\tc")]);

	expect(lines[0][0].text).toBe("ab  c");
});

test("a tab at the start of a line is a full stop wide", () => {
	const lines = layoutLines([token("\tindented")]);

	expect(lines[0][0].text).toBe("    indented");
});

test("carriage returns do not become their own line", () => {
	const lines = layoutLines([token("one\r\ntwo\r\n")]);

	expect(lines).toHaveLength(3);
	expect(lines[0][0].text).toBe("one");
});

test("the longest line decides how wide the content is", () => {
	const lines = layoutLines([token("ab\ncdef\ng")]);

	expect(longestLine(lines)).toBe(4);
});

test("only the lines around the viewport are drawn", () => {
	const range = visibleLines(200, 100, 20, 100);

	expect(range.first).toBe(9);
	expect(range.last).toBe(16);
});

test("the drawn range stops at the ends of the file", () => {
	const range = visibleLines(0, 100, 20, 3);

	expect(range).toEqual({ first: 0, last: 3 });
});

test("a viewport with no line height draws nothing", () => {
	expect(visibleLines(0, 100, 0, 50)).toEqual({ first: 0, last: 0 });
});

test("each class is listed once so its colour is looked up once", () => {
	const lines = layoutLines([token("a", "hl-k"), token("b", "hl-k"), token("c", "hl-s")]);

	expect(usedClasses(lines).sort()).toEqual(["hl-k", "hl-s"]);
});
