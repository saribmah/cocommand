import Ajv from "ajv";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const schemaPath = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "schema",
  "workflow.schema.json"
);

const workflowSchema = JSON.parse(readFileSync(schemaPath, "utf-8"));
const ajv = new Ajv({ allErrors: true, strict: false });
const validateFn = ajv.compile(workflowSchema);

export function getWorkflowSchema() {
  return workflowSchema;
}

export function validateWorkflow(workflow) {
  const valid = validateFn(workflow);
  return {
    valid: Boolean(valid),
    errors: validateFn.errors ?? [],
  };
}
