import { validateWorkflow } from "@cocommand/workflows";
import { WorkflowDefinition } from "./types";

export class WorkflowRegistry {
  #workflows = new Map<string, WorkflowDefinition>();

  register(workflow: WorkflowDefinition) {
    const validation = validateWorkflow(workflow);
    if (!validation.valid) {
      const message = validation.errors
        .map((error) => `${error.instancePath || "/"} ${error.message ?? ""}`)
        .join(", ");
      throw new Error(`Invalid workflow schema: ${message}`);
    }
    if (this.#workflows.has(workflow.id)) {
      throw new Error(`Workflow already registered: ${workflow.id}`);
    }
    this.#workflows.set(workflow.id, workflow);
  }

  registerAll(workflows: WorkflowDefinition[]) {
    workflows.forEach((workflow) => this.register(workflow));
  }

  getById(id: string) {
    return this.#workflows.get(id);
  }

  list() {
    return Array.from(this.#workflows.values());
  }
}
