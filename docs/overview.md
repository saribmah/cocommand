# Overview

Cocommand is an AI-native command bar for macOS. The app is meant to feel like Spotlight or Raycast, but with a brain: type a natural-language request, and the system interprets the intent, chooses a command or workflow, and returns a plan you can run.

The product has two goals:
- Make common tasks fast: a single entry point for sending messages, organizing files, or drafting quick notes.
- Make automation simple: users can chain commands into workflows without writing code.

## How it works

1. The user opens the command bar and types a request.
2. The backend planner classifies the intent and produces an execution plan.
3. If the intent maps to a workflow, the workflow runner resolves and executes its steps.
4. Results and plan details are shown in the UI.

## Key concepts

- **Command**: An atomic action the app can perform (e.g., “quick note”). Commands have a schema, inputs, and steps.
- **Workflow**: A chain of command IDs that represents a multi-step task (e.g., “daily wrap”).
- **Planner**: The intent classifier that maps user input to commands or workflows.

## Where to look

- UI and window behavior: `apps/desktop/src/`
- Tauri backend commands, planner, and workflow execution: `apps/desktop/src-tauri/`
- Backend server and application modules: `apps/desktop/src-tauri/src/`
- Architecture and product concept: `ARCHITECTURE.md`, `SPEC.md`
