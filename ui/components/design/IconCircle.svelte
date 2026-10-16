<!-- The design system's icon circle: an icon on a soft square tinted by its tone. -->
<script lang="ts">
	import { Icon, View } from "@neoworks/svelte-gpui/ui";
	import { colors, radius } from "../../theme";

	export type Tone = "green" | "red" | "amber" | "blue" | "violet" | "neutral";
	type Size = "sm" | "md" | "lg";

	let { icon, tone = "neutral", size = "md" }: { icon: string; tone?: Tone; size?: Size } = $props();

	const toneColors: Record<Tone, { background: string; color: string }> = {
		green: { background: colors.greenSoft, color: colors.green },
		red: { background: colors.redSoft, color: colors.red },
		amber: { background: colors.amberSoft, color: colors.amber },
		blue: { background: colors.blueSoft, color: colors.blue },
		violet: { background: colors.violetSoft, color: colors.violet },
		neutral: { background: colors.raised, color: colors.muted },
	};

	const boxSizes: Record<Size, { size: number; radius: number }> = {
		sm: { size: 32, radius: radius.md },
		md: { size: 40, radius: radius.md },
		lg: { size: 48, radius: radius.lg },
	};

	const iconSizes: Record<Size, number> = { sm: 16, md: 18, lg: 22 };
</script>

<View
	style={{
		...boxSizes[size],
		shrink: 0,
		align: "center",
		justify: "center",
		background: toneColors[tone].background,
	}}
>
	<Icon svg={icon} size={iconSizes[size]} color={toneColors[tone].color} />
</View>
