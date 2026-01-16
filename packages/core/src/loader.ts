import { readFileSync, readdirSync } from "node:fs";
import { extname, join } from "node:path";
import { WorkflowDefinition } from "./types";
import { WorkflowRegistry } from "./workflows";

export interface WorkflowLoadError {
  file: string;
  message: string;
}

export function loadWorkflowsFromDir(
  directory: string,
  registry: WorkflowRegistry
) {
  const errors: WorkflowLoadError[] = [];
  const workflows: WorkflowDefinition[] = [];
  const entries = readdirSync(directory, { withFileTypes: true });

  entries.forEach((entry) => {
    if (!entry.isFile() || extname(entry.name) !== ".json") {
      return;
    }

    const filePath = join(directory, entry.name);
    try {
      const raw = readFileSync(filePath, "utf-8");
      const workflow = JSON.parse(raw) as WorkflowDefinition;
      registry.register(workflow);
      workflows.push(workflow);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      errors.push({ file: filePath, message });
    }
  });

  return { workflows, errors };
}
