import { readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";
import { CommandDefinition } from "./types";
import { CommandRegistry } from "./commands";

export interface CommandLoadError {
  file: string;
  message: string;
}

export function loadCommandsFromDir(
  directory: string,
  registry: CommandRegistry
) {
  const errors: CommandLoadError[] = [];
  const commands: CommandDefinition[] = [];
  const entries = readdirSync(directory, { withFileTypes: true });

  entries.forEach((entry) => {
    if (!entry.isFile() || extname(entry.name) !== ".json") {
      return;
    }

    const filePath = join(directory, entry.name);
    try {
      const raw = readFileSync(filePath, "utf-8");
      const command = JSON.parse(raw) as CommandDefinition;
      registry.register(command);
      commands.push(command);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      errors.push({ file: filePath, message });
    }
  });

  return { commands, errors };
}
