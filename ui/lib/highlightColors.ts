/**
 * The colours of the classes a highlighted preview carries, grouped by kind
 * (keywords, strings, comments, …) and drawn with the palette's own tokens.
 */
import type { Style } from "@neoworks/svelte-gpui/ui";
import { colors } from "../theme";

/** The prefix the backend puts in front of every class. */
const classPrefix = "hl-";

const groups: { color: string; isItalic: boolean; classes: string[] }[] = [
	{ color: colors.amber, isItalic: false, classes: ["il", "l", "ld", "m", "mb", "mf", "mh", "mi", "mo", "no"] },
	{ color: colors.blue, isItalic: false, classes: ["bp", "fm", "nb", "nc", "ne", "nf", "nn"] },
	{
		color: colors.green,
		isItalic: false,
		classes: ["dl", "s", "s1", "s2", "sa", "sb", "sc", "sd", "se", "sh", "si", "sr", "ss", "sx"],
	},
	{ color: colors.pink, isItalic: false, classes: ["cp", "cpf", "nd", "ni", "nt"] },
	{ color: colors.red, isItalic: false, classes: ["err"] },
	{ color: colors.violet, isItalic: false, classes: ["k", "kc", "kd", "kn", "kp", "kr", "kt"] },
	{ color: colors.dim, isItalic: true, classes: ["c", "c1", "ch", "cm", "cs"] },
	{ color: colors.muted, isItalic: false, classes: ["o", "or", "ow", "p"] },
];

const stylesByClass = new Map<string, Style>();
for (const group of groups) {
	for (const name of group.classes) {
		stylesByClass.set(`${classPrefix}${name}`, { color: group.color, italic: group.isItalic });
	}
}

/** What a token of this class looks like. A class nothing colours is drawn in the text around it. */
export function styleOfTokenClass(className: string): Style {
	const style = stylesByClass.get(className);
	if (!style) {
		return {};
	}
	return style;
}
