---
title: v0 Milestone 12 Handoff
status: completed
---

# AI Implementation Handoff — UI-0

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`UI-0 — Tauri bootstrap + Core wiring`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add minimal glue code to bridge Tauri to Core.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Add UI components or frontend state changes beyond the invoke wiring.
- Add business logic inside Tauri; it must only bridge to core.
- Implement future milestones (UI-1+ or platform integrations).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-ui.md`

---

## Task List

Implement only the following tasks:

- [ ] Create a single shared `Core` instance in Tauri state (`Arc<Core>`).
- [ ] Add invoke handlers:
  - `submit_command`
  - `confirm_action`
  - `get_workspace_snapshot`
  - `get_recent_actions`

---

## Allowed Files / Folders

You may create or modify only the following:

```text
apps/desktop/src-tauri/src/main.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src-tauri/src/state.rs
apps/desktop/src-tauri/src/commands.rs
```

---

## Acceptance Criteria

- Frontend can call `submit_command` and get a response.

---

## Definition of Done

- Tauri layer contains no business logic beyond bridging.

---

## Tests

- Manual invoke from frontend (devtools) returns response.
