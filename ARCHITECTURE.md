# Architecture

## Goals
- Fast, low-friction command execution.
- Clear separation between UI, planning, and execution.
- Extensible integrations for third-party apps and OS services.
- Safe execution with auditability and opt-in automation.

## High-Level Flow
1) Capture: global hotkey opens the command bar.
2) Plan: AI selects an application and a tool (or uses the user-selected app).
3) Execute: the tool runs with validated inputs and permissions.
4) Feedback: results and traces are shown in the UI and stored locally.

## Core Concepts
- Application: a target surface like Spotify, Notetaker, or Finder. Each application exposes tools and capabilities.
- Tool: a single action the app can perform (e.g., `spotify.play`, `notes.create`, `finder.move`).
- Command: a user request expressed in natural language.
- Intent: structured decision about which application + tool to use and with what parameters.
- Workflow: a chain of tool calls (optionally across multiple applications).

## Planner Responsibilities
- If the user pre-selects an application, constrain planning to that application.
- Otherwise, classify the best application for the intent.
- Select the tool within that application and map inputs.
- Produce an execution plan with steps and permissions.

## Execution Model
- Tools are deterministic and typed. AI may fill parameters, but execution is explicit.
- Each tool declares inputs, outputs, and required permissions.
- Workflows are sequences of tool calls with error policies.

## Module Boundaries
- apps/desktop-tauri: UI, windowing, backend server bootstrap.
- apps/desktop-tauri/src-tauri/src/applications: application + tool definitions.
- apps/desktop-tauri/src-tauri/src/llm: LLM client and selection logic.
- apps/desktop-tauri/src-tauri/src/server: HTTP API surface.

## Extensibility
- Add a new application by defining tools + permissions in an integration module.
- Tools become available to the planner via the registry.
- Workflows can stitch tools across applications.

## Safety and Trust
- Default to read-only actions unless confirmed.
- Permission tiers per application and tool.
- Audit log of tool runs and results.
