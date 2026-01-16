export type CommandSource = "manual" | "workflow";

export interface CommandInput {
  id: string;
  text: string;
  source: CommandSource;
  createdAt: string;
}

export interface CommandContext {
  recentFiles?: string[];
  clipboardText?: string;
  calendarSummary?: string;
  activeApp?: string;
}

export interface Intent {
  id: string;
  name: string;
  confidence: number;
  parameters: Record<string, unknown>;
}

export type PlanStepStatus = "pending" | "running" | "completed" | "failed";

export interface PlanStep {
  id: string;
  tool: string;
  inputs: Record<string, unknown>;
  status: PlanStepStatus;
  outputs?: Record<string, unknown>;
}

export interface ExecutionPlan {
  id: string;
  intent: Intent;
  steps: PlanStep[];
  createdAt: string;
}

export interface ExecutionResult {
  planId: string;
  status: "ok" | "failed";
  summary: string;
  outputs?: Record<string, unknown>;
}

export interface ToolDefinition {
  id: string;
  name: string;
  description: string;
  inputsSchema: Record<string, unknown>;
  outputsSchema?: Record<string, unknown>;
  permissions?: string[];
}

export interface WorkflowInputSchemaEntry {
  type: string;
  description?: string;
  required?: boolean;
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  description?: string;
  version: string;
  inputs?: Record<string, WorkflowInputSchemaEntry>;
  steps: Array<{
    id: string;
    tool: string;
    inputs: Record<string, unknown>;
    outputs?: Record<string, unknown>;
    onError?: "halt" | "continue";
  }>;
  permissions?: Record<string, string>;
}
