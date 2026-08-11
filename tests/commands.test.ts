import { describe, expect, test } from "bun:test";
import { CommandRegistry, type Command } from "../frontend/src/lib/commands";

function counter(id: string, when?: string) {
	const calls: string[] = [];
	const command: Command = {
		id,
		title: id,
		category: "Test",
		when,
		run: () => calls.push(id),
	};
	return { command, calls };
}

describe("CommandRegistry", () => {
	test("a registered command runs", () => {
		const registry = new CommandRegistry();
		const { command, calls } = counter("file.copy");
		registry.register(command);
		expect(registry.run("file.copy", {})).toBe(true);
		expect(calls).toEqual(["file.copy"]);
	});

	test("an unknown command reports that it did not run", () => {
		const registry = new CommandRegistry();
		expect(registry.run("nothing.here", {})).toBe(false);
	});

	test("a command whose clause does not hold is not run", () => {
		const registry = new CommandRegistry();
		const { command, calls } = counter("file.copy", "selectionCount > 0");
		registry.register(command);
		expect(registry.run("file.copy", { selectionCount: 0 })).toBe(false);
		expect(calls).toEqual([]);
		expect(registry.run("file.copy", { selectionCount: 1 })).toBe(true);
	});

	test("enablement is reportable without running anything", () => {
		const registry = new CommandRegistry();
		const { command, calls } = counter("file.copy", "selectionCount > 0");
		registry.register(command);
		expect(registry.isEnabled("file.copy", { selectionCount: 0 })).toBe(false);
		expect(registry.isEnabled("file.copy", { selectionCount: 3 })).toBe(true);
		expect(registry.isEnabled("nothing.here", {})).toBe(false);
		expect(calls).toEqual([]);
	});

	test("lookup and listing return what was registered", () => {
		const registry = new CommandRegistry();
		registry.registerAll([counter("a").command, counter("b").command]);
		expect(registry.byId("a")?.title).toBe("a");
		expect(registry.all().map((command) => command.id)).toEqual(["a", "b"]);
	});
});

describe("withdrawing commands", () => {
	// The plugin host holds these so a crashed plugin's contributions can be
	// removed rather than left behind as entries that throw when invoked.
	test("a disposer removes the command it registered", () => {
		const registry = new CommandRegistry();
		const dispose = registry.register(counter("plugin.thing").command);
		dispose();
		expect(registry.byId("plugin.thing")).toBeUndefined();
		expect(registry.run("plugin.thing", {})).toBe(false);
	});

	test("registering all of them gives one disposer for the set", () => {
		const registry = new CommandRegistry();
		const dispose = registry.registerAll([counter("a").command, counter("b").command]);
		dispose();
		expect(registry.all()).toEqual([]);
	});

	// Otherwise a plugin unloading would take the replacement with it.
	test("a disposer does not remove a command that replaced it", () => {
		const registry = new CommandRegistry();
		const dispose = registry.register(counter("shared").command);
		const replacement = counter("shared");
		registry.register(replacement.command);
		dispose();
		expect(registry.byId("shared")).toBe(replacement.command);
	});
});
