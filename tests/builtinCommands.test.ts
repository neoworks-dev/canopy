import { describe, expect, test } from "bun:test";
import {
	backgroundMenuIds,
	defaultKeymap,
	movementCommands,
	selectionMenuIds,
	staticCommandDescriptors,
} from "../ui/lib/builtinCommands";
import { compileKeymap } from "../ui/lib/keymap";
import { whenClauseIsValid } from "../ui/lib/whenClause";

const builtinIds = new Set([
	...staticCommandDescriptors.map((descriptor) => descriptor.id),
	...movementCommands.map((command) => command.id),
]);

describe("the built-in command set", () => {
	test("no command id is defined twice", () => {
		const ids = [
			...staticCommandDescriptors.map((descriptor) => descriptor.id),
			...movementCommands.map((command) => command.id),
		];
		expect(ids).toHaveLength(builtinIds.size);
	});

	test("every clause parses, so no command silently disables itself", () => {
		const broken = staticCommandDescriptors.filter(
			(descriptor) => !whenClauseIsValid(descriptor.when ?? ""),
		);
		expect(broken.map((descriptor) => descriptor.id)).toEqual([]);
	});

	test("both a plain and an extending command exist for every movement", () => {
		expect(movementCommands.filter((command) => command.extend)).toHaveLength(8);
		expect(movementCommands.filter((command) => !command.extend)).toHaveLength(8);
	});
});

describe("the default keymap", () => {
	test("every key compiles, so no default binding is silently dropped", () => {
		expect(compileKeymap(defaultKeymap)).toHaveLength(defaultKeymap.length);
	});

	test("every binding points at a command that exists", () => {
		const dangling = defaultKeymap.filter((binding) => !builtinIds.has(binding.command));
		expect(dangling.map((binding) => `${binding.key} -> ${binding.command}`)).toEqual([]);
	});

	test("every clause parses, so no default binding is dead", () => {
		const broken = defaultKeymap.filter((binding) => !whenClauseIsValid(binding.when ?? ""));
		expect(broken.map((binding) => binding.key)).toEqual([]);
	});

	// Two bindings on one key under the same conditions means the earlier one is
	// unreachable, which is a mistake rather than a deliberate override.
	test("no two bindings shadow each other outright", () => {
		const seen = new Set<string>();
		const shadowed: string[] = [];
		for (const binding of compileKeymap(defaultKeymap)) {
			const signature = `${binding.chords.join(" ")}|${binding.when}`;
			appendIfSeen(seen, shadowed, signature);
		}
		expect(shadowed).toEqual([]);
	});

	test("the letters the modal layout promises are bound", () => {
		expect(commandFor("f")).toBe("search.here");
		expect(commandFor("z")).toBe("search.global");
		expect(commandFor("/")).toBe("list.find");
		expect(commandFor("Z")).toBe("zoxide.jump");
		expect(commandFor("h")).toBe("navigate.parent");
		expect(commandFor("l")).toBe("navigate.open");
	});
});

describe("the context menus", () => {
	const menuIds = [...selectionMenuIds.flat(), ...backgroundMenuIds.flat()];

	test("every menu entry names a command that exists", () => {
		expect(menuIds.filter((id) => !builtinIds.has(id))).toEqual([]);
	});
});

function commandFor(key: string): string {
	const bound = defaultKeymap.filter((binding) => binding.key === key);
	return bound[bound.length - 1].command;
}

function appendIfSeen(seen: Set<string>, shadowed: string[], signature: string) {
	if (seen.has(signature)) {
		shadowed.push(signature);
		return;
	}
	seen.add(signature);
}
