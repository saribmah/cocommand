import Ajv from "ajv";
import commandSchema from "../schema/command.schema.json" assert { type: "json" };
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
