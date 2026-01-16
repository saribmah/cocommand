import {
  CommandInput,
  ExecutionPlan,
  Intent,
  Planner,
  WorkflowDefinition,
} from "@cocommand/core";
import { CommandDefinition } from "@cocommand/core";
import { CommandRegistry } from "@cocommand/core";
import { WorkflowRegistry } from "@cocommand/core";

export interface PlannerConfig {
  commandRegistry: CommandRegistry;
  workflowRegistry: WorkflowRegistry;
}

export class HeuristicPlanner implements Planner {
  constructor(private config: PlannerConfig) {}

  async plan(command: CommandInput): Promise<ExecutionPlan> {
    const normalized = normalize(command.text);
    const commands = this.config.commandRegistry.list();
    const workflows = this.config.workflowRegistry.list();

    const bestCommand = bestMatch(commands, normalized);
    const bestWorkflow = bestMatch(workflows, normalized);

    if (bestWorkflow && (!bestCommand || bestWorkflow.score >= bestCommand.score)) {
      const intent = buildIntent(command, "workflow", bestWorkflow.item);
      return {
        id: `plan_${command.id}`,
        intent,
        steps: [
          {
            id: `step_${bestWorkflow.item.id}`,
            tool: "workflow.run",
            inputs: { workflowId: bestWorkflow.item.id },
            status: "pending",
          },
        ],
        createdAt: new Date().toISOString(),
      };
    }

    if (bestCommand) {
      const intent = buildIntent(command, "command", bestCommand.item);
      return {
        id: `plan_${command.id}`,
        intent,
        steps: [
          {
            id: `step_${bestCommand.item.id}`,
            tool: "command.run",
            inputs: { commandId: bestCommand.item.id },
            status: "pending",
          },
        ],
        createdAt: new Date().toISOString(),
      };
    }

    const intent: Intent = {
      id: `intent_${command.id}`,
      name: "freeform",
      confidence: 0.1,
      parameters: { text: command.text },
    };

    return {
      id: `plan_${command.id}`,
      intent,
      steps: [],
      createdAt: new Date().toISOString(),
    };
  }
}

function normalize(value: string) {
  return value.trim().toLowerCase();
}

function buildIntent(
  command: CommandInput,
  type: "command" | "workflow",
  match: CommandDefinition | WorkflowDefinition
): Intent {
  return {
    id: `intent_${command.id}`,
    name: match.name,
    confidence: 0.6,
    parameters: {
      type,
      id: match.id,
    },
  };
}

function bestMatch<T extends { name: string; description?: string }>(
  items: T[],
  query: string
) {
  let best: { item: T; score: number } | null = null;
  items.forEach((item) => {
    const score = scoreMatch(item, query);
    if (score <= 0) return;
    if (!best || score > best.score) {
      best = { item, score };
    }
  });
  return best;
}

function scoreMatch(
  item: { name: string; description?: string },
  query: string
) {
  if (!query) return 0;
  const name = normalize(item.name);
  const desc = normalize(item.description ?? "");

  if (name === query) return 1;
  if (name.includes(query)) return 0.8;
  if (desc.includes(query)) return 0.5;
  return 0;
}
