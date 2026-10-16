/**
 * Layout for the canvas the highlighted preview is drawn on. A file of any size
 * is one long run of tokens; this turns it into lines of positioned runs, so
 * drawing a frame only has to walk the lines that are on screen. Kept free of
 * the DOM so the geometry can be tested on its own.
 */
import type { highlight } from "../backend/models";

/** One stretch of text on one line, drawn in the colour its class carries. */
export type CodeRun = {
	text: string;
	column: number;
	className: string;
};

// What every editor this preview sits next to shows a tab as.
const tabWidth = 4;

export function layoutLines(tokens: highlight.Token[]): CodeRun[][] {
	const lines: CodeRun[][] = [[]];
	for (const token of tokens) {
		appendToken(lines, token);
	}
	return lines;
}

function appendToken(lines: CodeRun[][], token: highlight.Token) {
	const pieces = token.text.replaceAll("\r", "").split("\n");
	pieces.forEach((piece, index) => {
		startLineAfterTheFirstPiece(lines, index);
		appendPiece(lines[lines.length - 1], piece, token.class);
	});
}

function startLineAfterTheFirstPiece(lines: CodeRun[][], index: number) {
	if (index === 0) {
		return;
	}
	lines.push([]);
}

function appendPiece(line: CodeRun[], text: string, className: string) {
	if (text === "") {
		return;
	}
	const column = endColumn(line);
	line.push({ text: expandTabs(text, column), column, className });
}

function endColumn(line: CodeRun[]): number {
	const last = line[line.length - 1];
	if (!last) {
		return 0;
	}
	return last.column + last.text.length;
}

// Tabs are drawn as the spaces that reach the next stop rather than measured as
// a glyph, which is what makes a column a character offset everywhere else.
function expandTabs(text: string, column: number): string {
	let expanded = "";
	for (const character of text) {
		expanded += expandedCharacter(character, column + expanded.length);
	}
	return expanded;
}

function expandedCharacter(character: string, column: number): string {
	if (character !== "\t") {
		return character;
	}
	return " ".repeat(tabWidth - (column % tabWidth));
}

export function longestLine(lines: CodeRun[][]): number {
	return lines.reduce((longest, line) => Math.max(longest, endColumn(line)), 0);
}

/** The lines a viewport of this height shows, with one row of slack each way. */
export function visibleLines(
	scrollTop: number,
	viewportHeight: number,
	lineHeight: number,
	lineCount: number,
): { first: number; last: number } {
	if (lineHeight <= 0) {
		return { first: 0, last: 0 };
	}
	const first = Math.max(0, Math.floor(scrollTop / lineHeight) - 1);
	const last = Math.min(lineCount, Math.ceil((scrollTop + viewportHeight) / lineHeight) + 1);
	return { first, last };
}

/** The classes the lines use, so their colours are looked up once each. */
export function usedClasses(lines: CodeRun[][]): string[] {
	const classes = new Set<string>();
	for (const line of lines) {
		line.forEach((run) => classes.add(run.className));
	}
	return [...classes];
}
