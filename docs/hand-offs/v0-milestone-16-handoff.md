---
title: v0 Milestone 16 Handoff
status: done
---

# AI Implementation Handoff — Core-12

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-12 — Desktop Bridge Contract (Core API + Response Types)`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add unit tests only for the behaviors described in **Test Checklist**.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Change desktop UI components or Tauri wiring.
- Implement future milestones (UI-4+, platform wiring, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [x] Define stable request/response types used by the desktop UI:
  - `SubmitCommandRequest { text: String }`
  - `CoreResponse` (see response variants below)
  - `ConfirmActionRequest { confirmation_id: String, decision: bool }`
  - `ActionSummary` for Recent Actions UI
- [x] Implement `Core::submit_command(text)` and `Core::confirm_action(...)` to return **only** `CoreResponse`.
- [x] Ensure responses are fully serializable (serde) and contain no non-serializable types.

---

## CoreResponse (v0) — Required Variants

- `Artifact`: a shell-renderable result with optional actions
- `Preview`: read-only preview payload (e.g., last note)
- `Confirmation`: confirmation prompt payload (risk actions)
- `Error`: user-displayable error payload

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/core.rs
crates/cocommand/src/types.rs
crates/cocommand/src/error.rs
```

---

## Acceptance Criteria

- Core produces responses in a single stable shape for all command outcomes.
- Core responses are serializable via `serde` without custom hacks.
- Confirmation responses include a stable `confirmation_id` usable by UI.

---

## Definition of Done

- Desktop layer does not need to understand tools, workspace internals, or events to render results.
- CoreResponse is the only format used across the Tauri boundary.

---

## Test Checklist

- Unit test: `serde_json::to_string(&CoreResponse)` succeeds for each variant.
- Unit test: stub command produces each variant at least once.
