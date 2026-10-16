// The look of a card that floats over the window: menus, dropdowns and the
// command prompts. It is the design system's raised Card plus its large shadow.
import type { Style } from "@neoworks/svelte-gpui/ui";
import { colors, radius, shadow, space } from "../theme";

export const floatingPanel: Style = {
	background: colors.raised,
	radius: radius.lg,
	borderWidth: 1,
	borderColor: colors.lineFaint,
	padding: space[2],
	gap: space[2],
	shadow: shadow.lg,
	color: colors.text,
};
