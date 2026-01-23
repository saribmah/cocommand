---
title: Overview
---

Cocommand is an AI-native command bar built around explicit tooling, permissions, and a structured workspace.
These docs are the source of truth for product intent and implementation.

## Goals

- Fast, low-friction command execution.
- Clear separation between UI, planning, and execution.
- Extensible integrations for third-party apps and OS services.
- Safe execution with auditability and opt-in automation.

## Architecture Summary

1. User submits a command.
2. Capability router selects a small candidate set.
3. LLM planner proposes tool calls.
4. Permission layer authorizes or blocks.
5. Runtime executes tools and patches the workspace.
6. UI renders results and history.

## Start Here

- [Terminology](/spec/01-terminology/)
- [Execution Model](/spec/02-execution-model/)
- [Workspace](/spec/03-workspace/)
- [Permissions](/spec/04-permissions/)
- [Routing](/spec/05-routing/)
- [Extensions](/spec/06-extensions/)
- [Observability](/spec/07-observability/)
- [Built-ins](/builtins/)
- [UI Shell](/spec/09-ui-shell/)
- [Milestones](/spec/10-milestones/)
