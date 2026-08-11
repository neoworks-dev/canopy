/**
 * Every user-invokable thing in the explorer is a command. The context menu,
 * the keymap and — once NWFILES-10 lands — the palette all read this registry,
 * so a plugin that contributes a command reaches all three by registering once.
 *
 * A command's availability is a "when" clause rather than a predicate, because
 * a contributed command is described over RPC where a function cannot cross.
 */
import type { IconComponent } from "@neoworks-dev/ui";
import { evaluateWhen, type WhenContext } from "./whenClause";

export type CommandDescriptor = {
	id: string;
	title: string;
	/** Groups the command in the palette, and names its origin for a plugin. */
	category: string;
	icon?: IconComponent;
	/** Empty means the command is always available. */
	when?: string;
};

export type Command = CommandDescriptor & {
	run: () => void;
};

/** One rendered menu row: a command with its state already resolved. */
export type MenuItem = {
	id: string;
	title: string;
	icon: IconComponent;
	enabled: boolean;
	shortcut: string;
};

export class CommandRegistry {
	private commands = new Map<string, Command>();

	/**
	 * Returns a function that removes the command again. A plugin host holds
	 * these so a crashed plugin's contributions can be withdrawn rather than
	 * left behind as entries that throw when invoked.
	 */
	register(command: Command): () => void {
		this.commands.set(command.id, command);
		return () => this.unregister(command.id, command);
	}

	registerAll(commands: Command[]): () => void {
		const disposers = commands.map((command) => this.register(command));
		return () => disposers.forEach((dispose) => dispose());
	}

	byId(id: string): Command | undefined {
		return this.commands.get(id);
	}

	all(): Command[] {
		return [...this.commands.values()];
	}

	isEnabled(id: string, context: WhenContext): boolean {
		const command = this.commands.get(id);
		if (!command) {
			return false;
		}
		return commandApplies(command, context);
	}

	/** Runs the command if it exists and applies; reports whether it did. */
	run(id: string, context: WhenContext): boolean {
		const command = this.commands.get(id);
		if (!command || !commandApplies(command, context)) {
			return false;
		}
		command.run();
		return true;
	}

	// A later registration of the same id replaces this one, and must not then
	// be removed by the earlier registration's disposer.
	private unregister(id: string, registered: Command) {
		if (this.commands.get(id) !== registered) {
			return;
		}
		this.commands.delete(id);
	}
}

/** Whether a command applies right now. A command with no clause always does. */
export function commandApplies(command: Command, context: WhenContext): boolean {
	if (!command.when) {
		return true;
	}
	return evaluateWhen(command.when, context);
}
