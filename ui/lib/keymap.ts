/**
 * The keymap maps chord sequences to command ids. Bindings are a flat list in
 * priority order — built-ins first, then plugins, then the user's settings —
 * and the last binding that matches wins, so any layer can override the one
 * below it. A binding with an empty command removes what came before it.
 */
import { parseKeySequence, sequenceLabel } from "./keys";
import { evaluateWhen, type WhenContext } from "./whenClause";

export type Keybinding = {
	/** A chord, or a sequence of them separated by spaces: "ctrl+c", "g g". */
	key: string;
	/** Empty unbinds whatever earlier layers bound to this key. */
	command: string;
	when?: string;
};

export type CompiledBinding = {
	chords: string[];
	command: string;
	when: string;
};

export type ChordResolution =
	| { status: "none" }
	| { status: "pending" }
	| { status: "run"; command: string };

const noResolution: ChordResolution = { status: "none" };

/** Drops bindings whose key cannot be read rather than failing the whole map. */
export function compileKeymap(bindings: Keybinding[]): CompiledBinding[] {
	const compiled: CompiledBinding[] = [];
	for (const binding of bindings) {
		appendCompiled(compiled, binding);
	}
	return compiled;
}

/**
 * Decides what the next chord means, given the chords already pending.
 *
 * An exact match wins even when a longer sequence starts the same way. Waiting
 * to see whether the longer one arrives would mean a timeout, and a shortcut
 * that fires only after a pause is worse than one that cannot be shadowed.
 */
export function resolveChord(
	keymap: CompiledBinding[],
	pending: string[],
	chord: string,
	context: WhenContext,
): ChordResolution {
	const attempt = [...pending, chord];
	const candidates = keymap.filter((binding) => matches(binding, attempt, context));
	if (candidates.length === 0) {
		return noResolution;
	}
	return resolutionFor(candidates, attempt.length);
}

/** The label a menu or hint shows for a command, or "" when it has no binding. */
export function shortcutFor(
	keymap: CompiledBinding[],
	command: string,
	context: WhenContext,
): string {
	const bound = keymap.filter(
		(binding) => binding.command === command && evaluateWhen(binding.when, context),
	);
	if (bound.length === 0) {
		return "";
	}
	return sequenceLabel(bound[bound.length - 1].chords);
}

function resolutionFor(candidates: CompiledBinding[], length: number): ChordResolution {
	const exact = lastExactMatch(candidates, length);
	if (exact) {
		return runOrUnbind(exact);
	}
	return { status: "pending" };
}

function runOrUnbind(binding: CompiledBinding): ChordResolution {
	if (binding.command === "") {
		return noResolution;
	}
	return { status: "run", command: binding.command };
}

function lastExactMatch(candidates: CompiledBinding[], length: number): CompiledBinding | null {
	for (let index = candidates.length - 1; index >= 0; index--) {
		if (candidates[index].chords.length === length) {
			return candidates[index];
		}
	}
	return null;
}

function matches(binding: CompiledBinding, attempt: string[], context: WhenContext): boolean {
	if (!startsWith(binding.chords, attempt)) {
		return false;
	}
	return evaluateWhen(binding.when, context);
}

function startsWith(chords: string[], prefix: string[]): boolean {
	if (chords.length < prefix.length) {
		return false;
	}
	return prefix.every((chord, index) => chords[index] === chord);
}

function appendCompiled(compiled: CompiledBinding[], binding: Keybinding) {
	const chords = parseKeySequence(binding.key);
	if (chords.length === 0) {
		return;
	}
	compiled.push({ chords, command: binding.command, when: whenOf(binding) });
}

function whenOf(binding: Keybinding): string {
	if (!binding.when) {
		return "";
	}
	return binding.when;
}
