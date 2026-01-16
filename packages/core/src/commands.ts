import { validateCommand } from "@cocommand/commands";
import { CommandDefinition } from "./types";

export class CommandRegistry {
  #commands = new Map<string, CommandDefinition>();

  register(command: CommandDefinition) {
    const validation = validateCommand(command);
    if (!validation.valid) {
      const message = validation.errors
        .map((error) => `${error.instancePath || "/"} ${error.message ?? ""}`)
        .join(", ");
      throw new Error(`Invalid command schema: ${message}`);
    }
    if (this.#commands.has(command.id)) {
      throw new Error(`Command already registered: ${command.id}`);
    }
    this.#commands.set(command.id, command);
  }

  registerAll(commands: CommandDefinition[]) {
    commands.forEach((command) => this.register(command));
  }

  getById(id: string) {
    return this.#commands.get(id);
  }

  list() {
    return Array.from(this.#commands.values());
  }

  search(query: string) {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return this.list();
    }
    return this.list().filter(
      (command) =>
        command.name.toLowerCase().includes(normalized) ||
        command.description?.toLowerCase().includes(normalized)
    );
  }
}
