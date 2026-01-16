import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { validateWorkflow } from "@cocommand/workflows";

const examplePath = resolve(
  "packages",
  "workflows",
  "examples",
  "daily-wrap.json"
);

const workflow = JSON.parse(readFileSync(examplePath, "utf-8"));
const result = validateWorkflow(workflow);

if (!result.valid) {
  console.error("Workflow validation failed:");
  result.errors.forEach((error) => {
    console.error(`- ${error.instancePath || "/"} ${error.message ?? ""}`);
  });
  process.exit(1);
}

console.log("Workflow validation passed.");
