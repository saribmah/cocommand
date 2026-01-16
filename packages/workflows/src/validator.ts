import Ajv from "ajv";
import workflowSchema from "../schema/workflow.schema.json" assert { type: "json" };
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
