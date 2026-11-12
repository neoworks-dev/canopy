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
