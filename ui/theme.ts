// The Neoworks design tokens (`@neoworks-dev/ui` styles/tokens.css, dark theme) as
// plain data for style objects.
import type { Shadow, Style } from "@neoworks/svelte-gpui/ui";

export const colors = {
	canvas: "#141416",
	elevated: "#1c1c1e",
	surface: "#1c1c1e",
	raised: "#262628",
	hover: "#2e2e31",
	input: "#0e0e10",

	line: "#29292b",
	lineStrong: "#3f3f46",
	lineFaint: "rgba(255, 255, 255, 0.06)",

	text: "#fafafa",
	muted: "#a1a1aa",
	dim: "#71717a",
	faint: "#52525b",
	inverse: "#0b0b0d",

	action: "#fafafa",
	actionHover: "#e4e4e7",
	actionForeground: "#0b0b0d",

	focusRing: "rgba(250, 250, 250, 0.55)",
	scrim: "rgba(0, 0, 0, 0.66)",

	green: "#4ade80",
	red: "#f87171",
	amber: "#fbbf24",
	blue: "#60a5fa",
	pink: "#f472b6",
	violet: "#a78bfa",
	cyan: "#22d3ee",
	greenSoft: "rgba(74, 222, 128, 0.14)",
	redSoft: "rgba(248, 113, 113, 0.14)",
	amberSoft: "rgba(251, 191, 36, 0.14)",
	blueSoft: "rgba(96, 165, 250, 0.14)",
	pinkSoft: "rgba(244, 114, 182, 0.14)",
	violetSoft: "rgba(167, 139, 250, 0.14)",
	cyanSoft: "rgba(34, 211, 238, 0.14)",
	redBorder: "rgba(248, 113, 113, 0.3)",
	redHover: "rgba(248, 113, 113, 0.2)",
} as const;

/** The 4px spacing scale: `space[3]` is 12. */
export const space = {
	0.5: 2,
	1: 4,
	1.5: 6,
	2: 8,
	2.5: 10,
	3: 12,
	4: 16,
	5: 20,
	6: 24,
	8: 32,
	10: 40,
	12: 48,
} as const;

export const radius = {
	xs: 4,
	sm: 6,
	md: 8,
	lg: 12,
	xl: 16,
	full: 9999,
} as const;

export const textSize = {
	"2xs": 11,
	xs: 12,
	sm: 13,
	base: 14,
	md: 16,
	lg: 18,
	xl: 20,
	"2xl": 24,
} as const;

export const shadow: Record<"xs" | "sm" | "md" | "lg", Shadow[]> = {
	xs: [{ y: 1, blur: 2, color: "rgba(0, 0, 0, 0.3)" }],
	sm: [
		{ y: 1, blur: 3, color: "rgba(0, 0, 0, 0.4)" },
		{ y: 1, blur: 2, color: "rgba(0, 0, 0, 0.3)" },
	],
	md: [{ y: 4, blur: 12, color: "rgba(0, 0, 0, 0.45)" }],
	lg: [{ y: 12, blur: 32, color: "rgba(0, 0, 0, 0.55)" }],
};

export const duration = {
	fast: 120,
	base: 180,
	slow: 280,
} as const;

/** What the window and every text inside it start from. */
export const root: Style = {
	flex: 1,
	background: colors.canvas,
	color: colors.text,
	fontFamily: "sans",
	fontSize: textSize.base,
};

export const monoText: Style = {
	fontFamily: "mono",
	fontSize: textSize.sm,
};

/** A field's box: the input surface, a faint line, and a stronger one while focused. */
export const field: Style = {
	paddingX: space[2.5],
	paddingY: space[1.5],
	radius: radius.sm,
	background: colors.input,
	borderWidth: 1,
	borderColor: colors.line,
	fontSize: textSize.sm,
	color: colors.text,
};

export const fieldFocus: Style = {
	borderColor: colors.lineStrong,
};

/** A full-window dimmed backdrop for modal dialogs. */
export const scrim: Style = {
	position: "absolute",
	inset: 0,
	background: colors.scrim,
	align: "center",
	justify: "center",
};
