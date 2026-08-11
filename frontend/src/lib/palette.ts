/**
 * Ranking for the command palette. The palette offers only what the current
 * context allows, so a row that cannot run is never on screen, and it matches
 * loosely: the query's characters need only appear in order, which is what lets
 * "ntrm" reach "Open terminal here".
 */
import { commandApplies, type Command } from "./commands";
import type { WhenContext } from "./whenClause";

export type PaletteMatch = {
	command: Command;
	score: number;
	/** Indices into the command's title of the characters the query matched. */
	highlights: number[];
};

export type TitleSegment = { text: string; matched: boolean };

/** A run of matched characters scores more than the same characters scattered. */
const adjacentBonus = 8;
/** The start of the title, or of a word inside it, is what people type first. */
const boundaryBonus = 6;
const gapPenalty = 1;
const longestPenalizedGap = 6;
/** A command found only through its category ranks below one found by title. */
const categoryPenalty = 20;

export function availableCommands(commands: Command[], context: WhenContext): Command[] {
	return commands.filter((command) => commandApplies(command, context));
}

/** Ranks the commands a query reaches, best first. An empty query keeps them all. */
export function rankCommands(commands: Command[], query: string): PaletteMatch[] {
	const matches: PaletteMatch[] = [];
	for (const command of commands) {
		appendMatch(matches, command, query.trim().toLowerCase());
	}
	return matches.sort(byScoreThenTitle);
}

function appendMatch(matches: PaletteMatch[], command: Command, query: string) {
	const match = matchCommand(command, query);
	if (!match) {
		return;
	}
	matches.push(match);
}

function matchCommand(command: Command, query: string): PaletteMatch | null {
	if (query === "") {
		return { command, score: 0, highlights: [] };
	}
	const inTitle = fuzzyScore(command.title, query);
	if (inTitle) {
		return { command, score: inTitle.score, highlights: inTitle.positions };
	}
	return categoryMatch(command, query);
}

// The category is searched with the title appended, so "navigation home" reads
// as one phrase. Only the title carries highlights, so none survive this.
function categoryMatch(command: Command, query: string): PaletteMatch | null {
	const inCategory = fuzzyScore(`${command.category} ${command.title}`, query);
	if (!inCategory) {
		return null;
	}
	return { command, score: inCategory.score - categoryPenalty, highlights: [] };
}

function byScoreThenTitle(first: PaletteMatch, second: PaletteMatch): number {
	if (first.score !== second.score) {
		return second.score - first.score;
	}
	return first.command.title.localeCompare(second.command.title);
}

type Scan = { score: number; positions: number[]; previousIndex: number };

type Scored = { score: number; positions: number[] };

/**
 * Greedy leftmost subsequence match. Leftmost is not always the highest scoring
 * placement, but it is O(n) and the titles here are short enough that the
 * difference does not show.
 */
function fuzzyScore(text: string, query: string): Scored | null {
	const haystack = text.toLowerCase();
	let scan: Scan = { score: 0, positions: [], previousIndex: -1 };
	for (const character of query) {
		const advanced = advance(haystack, character, scan);
		if (!advanced) {
			return null;
		}
		scan = advanced;
	}
	return { score: scan.score, positions: scan.positions };
}

function advance(haystack: string, character: string, scan: Scan): Scan | null {
	const index = haystack.indexOf(character, scan.previousIndex + 1);
	if (index < 0) {
		return null;
	}
	return {
		score: scan.score + characterScore(haystack, index, scan.previousIndex),
		positions: [...scan.positions, index],
		previousIndex: index,
	};
}

function characterScore(haystack: string, index: number, previousIndex: number): number {
	if (index === previousIndex + 1) {
		return adjacentBonus;
	}
	return placementBonus(haystack, index) - gapCost(index, previousIndex);
}

function placementBonus(haystack: string, index: number): number {
	if (index === 0) {
		return boundaryBonus;
	}
	if (haystack[index - 1] === " " || haystack[index - 1] === ".") {
		return boundaryBonus;
	}
	return 0;
}

function gapCost(index: number, previousIndex: number): number {
	const gap = Math.min(index - previousIndex - 1, longestPenalizedGap);
	return gap * gapPenalty;
}

/** Splits a title into the runs the query matched and the runs it did not. */
export function titleSegments(title: string, highlights: number[]): TitleSegment[] {
	const matched = new Set(highlights);
	const segments: TitleSegment[] = [];
	for (let index = 0; index < title.length; index += 1) {
		appendCharacter(segments, title[index], matched.has(index));
	}
	return segments;
}

function appendCharacter(segments: TitleSegment[], character: string, matched: boolean) {
	const last = segments[segments.length - 1];
	if (last && last.matched === matched) {
		last.text += character;
		return;
	}
	segments.push({ text: character, matched });
}
