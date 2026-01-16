import Ajv from "ajv";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const schemaPath = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "schema",
  "command.schema.json"
);

const commandSchema = JSON.parse(readFileSync(schemaPath, "utf-8"));
const ajv = new Ajv({ allErrors: true, strict: false });
const validateFn = ajv.compile(commandSchema);

export function getCommandSchema() {
  return commandSchema;
}

export function validateCommand(command: unknown) {
  const valid = validateFn(command);
  return {
    valid: Boolean(valid),
    errors: validateFn.errors ?? [],
  };
}
