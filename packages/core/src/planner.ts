import { CommandContext, CommandInput, ExecutionPlan, Intent } from "./types";

export interface Planner {
  plan(command: CommandInput, context?: CommandContext): Promise<ExecutionPlan>;
}

export type IntentClassifier = (
  command: CommandInput,
  context?: CommandContext
) => Promise<Intent>;

export class DefaultPlanner implements Planner {
  constructor(private classifyIntent: IntentClassifier) {}

  async plan(command: CommandInput, context?: CommandContext): Promise<ExecutionPlan> {
    const intent = await this.classifyIntent(command, context);
    return {
      id: `plan_${command.id}`,
      intent,
      steps: [],
      createdAt: new Date().toISOString(),
    };
  }
}
