# Architecture

## Goals
- Fast, low-friction command execution.
- Extensible commands, workflows, and integrations.
- Clear boundaries between UI, orchestration, and system adapters.
- Safe execution with auditability and opt-in automation.

## High-Level Flow
1) Capture: global hotkey opens command bar.
2) Intent: LLM parses intent + context into a structured plan.
3) Plan: planner selects commands, workflows, tools, and steps.
4) Execute: integrations perform actions with user confirmation when needed.
5) Feedback: status, results, and traces shown in UI and stored locally.

## Core Domains
- Command: raw user input and command metadata.
- Context: local state (recent files, clipboard, calendar, etc.).
- Intent: structured representation of what the user wants.
- Command: reusable, parameterized action composed of tool steps.
- Workflow: chain of commands.
- Execution: tool calls and side effects.
- Memory: short-term session + optional long-term preference state.

## Package Boundaries
- apps/desktop-tauri: UI + OS shell.
- packages/core: domain types, routing, planner interfaces.
- packages/llm: model providers, prompts, parsing, safety rules.
- packages/commands: command registry, schema, validation.
- packages/integrations: adapters for OS and third-party services.
- packages/storage: local persistence and search.
- packages/sync: cross-device sync primitives (future).
- packages/shared: shared utils and types.

## Command and Workflow Model
Commands are declarative steps with typed inputs/outputs.
- Each step maps to a tool provided by integrations.
- LLM can fill parameters but execution is deterministic.
- User-defined commands are validated against schema.

Workflows chain commands together for higher-level automation.

## Execution Safety
- Default: read-only unless user confirms.
- Tiered permissions per command, workflow, and tool.
- Audit log for actions and results.

## Extensibility
- Integrations expose tool definitions and capabilities.
- Commands compose tools with explicit inputs/outputs.
- Planner chooses commands/workflows/tools based on intent and context.

## Future Sync
- Event log as source of truth.
- Sync service merges logs and reconciles conflicts.
