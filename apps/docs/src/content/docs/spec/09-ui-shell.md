---
title: UI Shell
---

The command bar is the primary UI surface. It stays lightweight and task-focused.

## Command Bar States

- Idle: ready for input.
- Planning: shows progress while routing/planning.
- Confirmation: shows actions requiring consent.
- Executing: shows tool execution progress.
- Result: displays outcome summary and follow-up suggestions.

## Result Preview (Ephemeral)

Results are shown inline and kept lightweight.
Follow-up hints appear when a command can be modified (e.g., “Make it 2:30”).

## Non-Headless App Panels

Some built-ins (Notes, Clipboard) expose UI panels within the workspace.
Panels are opened and focused via Kernel Tools and may be visible or hidden.
