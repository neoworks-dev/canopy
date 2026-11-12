import { describe, expect, test } from "bun:test";
import { modeLabel, percentLabel, permissionCharacters, positionLabel } from "../ui/lib/statusBar";

describe("the status bar", () => {
	test("names the mode as Yazi does", () => {
		expect(modeLabel("normal")).toBe("NOR");
		expect(modeLabel("select")).toBe("SEL");
		expect(modeLabel("unset")).toBe("UNS");
	});

	test("counts the cursor's place from one", () => {
		expect(positionLabel(2, 42)).toBe("3/42");
		expect(positionLabel(-1, 42)).toBe("");
		expect(positionLabel(0, 0)).toBe("");
	});

	test("says Top, Bot and All at the ends, a percentage between", () => {
		expect(percentLabel(0, 10)).toBe("Top");
		expect(percentLabel(9, 10)).toBe("Bot");
		expect(percentLabel(0, 1)).toBe("All");
		expect(percentLabel(3, 7)).toBe("50%");
	});

	test("colours each permission by what it grants", () => {
		const tones = permissionCharacters("drwxr-x--x").map((part) => part.tone);
		expect(tones).toEqual(["type", "read", "write", "execute", "read", "none", "execute", "none", "none", "execute"]);
	});
});
