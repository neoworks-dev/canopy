/**
 * Chords: one key press with its modifiers, normalized to a single string so a
 * keymap can be a lookup rather than a chain of comparisons.
 *
 * Shift is only part of a chord for named keys. A printable character already
 * carries it — "Z" is shift+z, "?" is shift+/ — and which physical key produced
 * that character is a layout question, so the character itself is what gets
 * matched. The cost is that a shifted digit has to be written as the character
 * it produces: "ctrl+!" rather than "ctrl+shift+1".
 */

export type ChordEvent = {
	key: string;
	ctrlKey: boolean;
	altKey: boolean;
	shiftKey: boolean;
	metaKey: boolean;
};

type ChordParts = {
	ctrl: boolean;
	alt: boolean;
	shift: boolean;
	meta: boolean;
	key: string;
};

/** Held on their own these produce a keydown that is not a chord yet. */
const modifierOnlyKeys = ["control", "alt", "shift", "meta", "altgraph", "capslock"];

const namedKeyAliases: Array<[string, string]> = [
	["escape", "escape"],
	["esc", "escape"],
	["enter", "enter"],
	["return", "enter"],
	["tab", "tab"],
	["backspace", "backspace"],
	["delete", "delete"],
	["del", "delete"],
	["insert", "insert"],
	["home", "home"],
	["end", "end"],
	["pageup", "pageup"],
	["pagedown", "pagedown"],
	["arrowup", "up"],
	["up", "up"],
	["arrowdown", "down"],
	["down", "down"],
	["arrowleft", "left"],
	["left", "left"],
	["arrowright", "right"],
	["right", "right"],
	["space", "space"],
];

const namedKeys = new Map<string, string>([...namedKeyAliases, ...functionKeyAliases()]);

type ChordModifier = "ctrl" | "alt" | "shift" | "meta";

const modifierAliases = new Map<string, ChordModifier>([
	["ctrl", "ctrl"],
	["control", "ctrl"],
	["alt", "alt"],
	["option", "alt"],
	["shift", "shift"],
	["meta", "meta"],
	["cmd", "meta"],
	["command", "meta"],
	["super", "meta"],
]);

const displayNames = new Map<string, string>([
	["escape", "Esc"],
	["enter", "Enter"],
	["tab", "Tab"],
	["backspace", "Backspace"],
	["delete", "Delete"],
	["insert", "Insert"],
	["home", "Home"],
	["end", "End"],
	["pageup", "Page Up"],
	["pagedown", "Page Down"],
	["up", "↑"],
	["down", "↓"],
	["left", "←"],
	["right", "→"],
	["space", "Space"],
]);

/** Turns a key event into a chord, or "" when the press cannot be one. */
export function chordFromEvent(event: ChordEvent): string {
	if (modifierOnlyKeys.includes(event.key.toLowerCase())) {
		return "";
	}
	return buildChord({
		ctrl: event.ctrlKey,
		alt: event.altKey,
		shift: event.shiftKey,
		meta: event.metaKey,
		key: event.key,
	});
}

/** Reads a written chord such as "ctrl+shift+n", or "" when it is unusable. */
export function parseChord(spec: string): string {
	const trimmed = spec.trim();
	if (trimmed === "") {
		return "";
	}
	const parts = splitSpec(trimmed);
	if (!parts) {
		return "";
	}
	return buildChord(parts);
}

/**
 * Reads a whole binding, which may be a sequence such as "g g". An empty array
 * means the binding could not be read and should be ignored rather than
 * silently matching nothing.
 */
export function parseKeySequence(spec: string): string[] {
	const specs = spec.trim().split(/\s+/).filter(Boolean);
	const chords = specs.map(parseChord);
	if (chords.length === 0 || chords.includes("")) {
		return [];
	}
	return chords;
}

/** Renders a chord for a menu or a shortcut hint. */
export function chordLabel(chord: string): string {
	const parts = splitSpec(chord);
	if (!parts) {
		return "";
	}
	return labelPrefix(parts) + labelForKey(parts.key);
}

export function sequenceLabel(chords: string[]): string {
	return chords.map(chordLabel).join(" ");
}

function buildChord(parts: ChordParts): string {
	const key = canonicalKey(parts.key);
	if (key === "") {
		return "";
	}
	const shifted = applyShiftToCharacter(key, parts.shift);
	return modifierPrefix(parts, shifted) + shifted;
}

/**
 * A written "shift+z" and a typed "Z" have to reach the same chord, so shift
 * folds into the character it would have produced.
 */
function applyShiftToCharacter(key: string, shift: boolean): string {
	if (!shift || !isPrintable(key)) {
		return key;
	}
	return key.toUpperCase();
}

function canonicalKey(key: string): string {
	if (key === " ") {
		return "space";
	}
	if (isPrintable(key)) {
		return key;
	}
	const named = namedKeys.get(key.toLowerCase());
	if (!named) {
		return "";
	}
	return named;
}

function modifierPrefix(parts: ChordParts, key: string): string {
	const prefixes: string[] = [];
	appendIf(prefixes, parts.ctrl, "ctrl+");
	appendIf(prefixes, parts.alt, "alt+");
	appendIf(prefixes, parts.shift && !isPrintable(key), "shift+");
	appendIf(prefixes, parts.meta, "meta+");
	return prefixes.join("");
}

function labelPrefix(parts: ChordParts): string {
	const prefixes: string[] = [];
	appendIf(prefixes, parts.ctrl, "Ctrl+");
	appendIf(prefixes, parts.alt, "Alt+");
	appendIf(prefixes, parts.shift || isShiftedLetter(parts.key), "Shift+");
	appendIf(prefixes, parts.meta, "Super+");
	return prefixes.join("");
}

// An uppercase letter is a shifted press, so the label says so even though the
// chord itself carries the shift in the character.
function isShiftedLetter(key: string): boolean {
	if (!isPrintable(key) || key.toUpperCase() === key.toLowerCase()) {
		return false;
	}
	return key === key.toUpperCase();
}

function labelForKey(key: string): string {
	const named = displayNames.get(key);
	if (named) {
		return named;
	}
	return key.toUpperCase();
}

function appendIf(target: string[], condition: boolean, value: string) {
	if (!condition) {
		return;
	}
	target.push(value);
}

/**
 * Splits "ctrl+shift+n" into its modifiers and its key. The key may itself be a
 * "+", which is why the last segment is taken by position rather than by
 * splitting on every separator.
 */
function splitSpec(spec: string): ChordParts | null {
	const segments = spec.split("+").map((segment) => segment.trim());
	const key = keySegment(segments);
	if (key === "") {
		return null;
	}
	return modifiersFrom(segments.slice(0, segments.length - keySegmentWidth(segments)), key);
}

function keySegment(segments: string[]): string {
	const last = segments[segments.length - 1];
	if (last !== "") {
		return last;
	}
	if (segments.length < 2) {
		return "";
	}
	return "+";
}

// A trailing "+" key arrives as an empty final segment preceded by an empty
// separator, so it consumes two segments rather than one.
function keySegmentWidth(segments: string[]): number {
	if (segments[segments.length - 1] === "") {
		return 2;
	}
	return 1;
}

function modifiersFrom(names: string[], key: string): ChordParts | null {
	const parts: ChordParts = { ctrl: false, alt: false, shift: false, meta: false, key };
	for (const name of names) {
		const modifier = modifierAliases.get(name.toLowerCase());
		if (!modifier) {
			return null;
		}
		parts[modifier] = true;
	}
	return parts;
}

function isPrintable(key: string): boolean {
	return [...key].length === 1;
}

function functionKeyAliases(): Array<[string, string]> {
	const pairs: Array<[string, string]> = [];
	for (let number = 1; number <= 12; number++) {
		pairs.push([`f${number}`, `f${number}`]);
	}
	return pairs;
}
