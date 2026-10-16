import { describe, expect, test } from "bun:test";
import { styleOfTokenClass } from "../ui/lib/highlightColors";
import { colors } from "../ui/theme";

describe("the colour of a highlighted token", () => {
	test("keywords, strings and numbers each have their own colour", () => {
		expect(styleOfTokenClass("hl-kn").color).toBe(colors.violet);
		expect(styleOfTokenClass("hl-s2").color).toBe(colors.green);
		expect(styleOfTokenClass("hl-mi").color).toBe(colors.amber);
	});

	test("a comment is drawn back and slanted", () => {
		expect(styleOfTokenClass("hl-c1")).toEqual({ color: colors.dim, italic: true });
	});

	test("punctuation is quieter than the text", () => {
		expect(styleOfTokenClass("hl-p").color).toBe(colors.muted);
	});

	test("text no class colours is left to the text around it", () => {
		expect(styleOfTokenClass("")).toEqual({});
		expect(styleOfTokenClass("hl-zz")).toEqual({});
	});
});
