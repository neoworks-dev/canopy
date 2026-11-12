/**
 * The rows of the help panel: every key that does something, as Yazi's `~`
 * lists them. A binding a later layer overrides or unbinds is not listed.
 */
import type { CompiledBinding } from "./keymap";
import { sequenceLabel } from "./keys";

export type HelpRow = {
	keys: string;
	command: string;
	title: string;
};

export function helpRows(keymap: CompiledBinding[], titleOf: (command: string) => string): HelpRow[] {
	return effectiveBindings(keymap).map((binding) => ({
		keys: sequenceLabel(binding.chords),
		command: binding.command,
		title: titleOf(binding.command),
	}));
}

/**
 * What can follow the chords typed so far, as Yazi's which-key popup lists
 * them after `g` or `,`: the next key of each sequence that still applies.
 */
export function continuationRows(
	keymap: CompiledBinding[],
	pending: string[],
	applies: (binding: CompiledBinding) => boolean,
	titleOf: (command: string) => string,
): HelpRow[] {
	const continuing = effectiveBindings(keymap).filter(
		(binding) => continues(binding.chords, pending) && applies(binding),
	);
	return continuing.map((binding) => ({
		keys: sequenceLabel(binding.chords.slice(pending.length)),
		command: binding.command,
		title: titleOf(binding.command),
	}));
}

function continues(chords: string[], pending: string[]): boolean {
	if (chords.length <= pending.length) {
		return false;
	}
	return pending.every((chord, index) => chords[index] === chord);
}

/** Matches the keys or the title, ignoring case, so both "g h" and "home" find "Go home". */
export function filterHelpRows(rows: HelpRow[], query: string): HelpRow[] {
	const needle = query.trim().toLowerCase();
	if (needle === "") {
		return rows;
	}
	return rows.filter((row) => matchesQuery(row, needle));
}

function matchesQuery(row: HelpRow, needle: string): boolean {
	if (row.keys.toLowerCase().includes(needle)) {
		return true;
	}
	return row.title.toLowerCase().includes(needle);
}

// The last binding for a key under the same condition wins, which is how the
// keymap resolves them too.
function effectiveBindings(keymap: CompiledBinding[]): CompiledBinding[] {
	const latest = new Map<string, CompiledBinding>();
	for (const binding of keymap) {
		latest.set(`${binding.chords.join(" ")}|${binding.when}`, binding);
	}
	return [...latest.values()].filter((binding) => binding.command !== "");
}
