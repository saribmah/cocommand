---
title: v0 Milestone 17 Handoff
status: done
---

# AI Implementation Handoff — UI-0A

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`UI-0A — Core Bridge Integration (Tauri invoke contract)`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Adjust types to match the Core bridge contract.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Add new UI features beyond contract alignment.
- Change core behavior or introduce new backend routes.

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-ui.md`

---

## Task List

Implement only the following tasks:

- [x] Implement Tauri invoke handlers that exactly mirror the Core bridge contract:
  - `submit_command(text) -> CoreResponse`
  - `confirm_action(confirmation_id, decision) -> CoreResponse`
  - `get_recent_actions(limit) -> Vec<ActionSummary>`
  - `get_workspace_snapshot() -> Workspace` (optional for v0 UI, but useful for debugging)
- [x] Ensure the frontend uses these invoke calls as the only backend integration path.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
apps/desktop/src-tauri/src/commands.rs
apps/desktop/src-tauri/src/state.rs
apps/desktop/src/types/core.ts
```

---

## Acceptance Criteria

- Frontend can submit a command and receive a `CoreResponse` without shape mismatches.
- Confirmation response round-trip works (UI → confirm_action → updated response).

---

## Definition of Done

- UI and Core share a stable boundary: no tool IDs leak into UI.
- All backend calls are routed through the Tauri commands layer.

---

## Test Checklist

- Manual: run a command and render Artifact.
- Manual: run destructive action and confirm.
