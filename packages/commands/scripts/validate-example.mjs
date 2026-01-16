import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { validateCommand } from "@cocommand/commands";

const examplePath = resolve(
  "packages",
  "commands",
  "examples",
  "quick-note.json"
);

const command = JSON.parse(readFileSync(examplePath, "utf-8"));
const result = validateCommand(command);

if (!result.valid) {
  console.error("Command validation failed:");
  result.errors.forEach((error) => {
    console.error(`- ${error.instancePath || "/"} ${error.message ?? ""}`);
  });
  process.exit(1);
}

console.log("Command validation passed.");
