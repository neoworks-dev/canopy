/**
 * What Yazi's status bar says: the mode on the left, the hovered entry's
 * permissions and the cursor's place in the folder on the right.
 */
import type { PaneStatus } from "./paneStatus";

export type PermissionTone = "type" | "read" | "write" | "execute" | "none";

export type PermissionCharacter = { character: string; tone: PermissionTone };

const modeLabels: Record<PaneStatus["mode"], string> = {
	normal: "NOR",
	select: "SEL",
	unset: "UNS",
};

export function modeLabel(mode: PaneStatus["mode"]): string {
	return modeLabels[mode];
}

/** "3/42", counting from one; empty when there is no cursor. */
export function positionLabel(cursorIndex: number, count: number): string {
	if (cursorIndex < 0 || count === 0) {
		return "";
	}
	return `${cursorIndex + 1}/${count}`;
}

/** Where the cursor is as Vim says it: Top, Bot, All for a single entry, else a percentage. */
export function percentLabel(cursorIndex: number, count: number): string {
	if (cursorIndex < 0 || count === 0) {
		return "";
	}
	if (count === 1) {
		return "All";
	}
	if (cursorIndex === 0) {
		return "Top";
	}
	if (cursorIndex === count - 1) {
		return "Bot";
	}
	return `${Math.round((cursorIndex / (count - 1)) * 100)}%`;
}

/** Each character of an `ls -l` mode string with the colour Yazi gives it. */
export function permissionCharacters(mode: string): PermissionCharacter[] {
	return [...mode].map((character, index) => ({ character, tone: toneOf(character, index) }));
}

function toneOf(character: string, index: number): PermissionTone {
	if (character === "-") {
		return "none";
	}
	if (index === 0) {
		return "type";
	}
	return toneOfPermission(character);
}

function toneOfPermission(character: string): PermissionTone {
	if (character === "r") {
		return "read";
	}
	if (character === "w") {
		return "write";
	}
	return "execute";
}
