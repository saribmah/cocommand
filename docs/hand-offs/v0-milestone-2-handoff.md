---
title: v0 Milestone 2 Handoff
status: completed
---

# AI Implementation Handoff — Core-2

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-2 — Events & Observability v0 (event stream + invocation records)`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add tests only for the behavior described in **Acceptance Criteria**.
- Use existing naming conventions and module layout rules.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Introduce new crates or services.
- Add UI logic or Tauri dependencies.
- Implement later milestones (tool system, routing, permissions, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/03-workspace.md`
- `apps/docs/src/content/docs/spec/07-observability.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define canonical event types:
  - `UserMessage`
  - `ToolCallProposed`
  - `ToolCallAuthorized`
  - `ToolCallDenied`
  - `ToolCallExecuted`
  - `ToolResultRecorded`
  - `WorkspacePatched`
  - `ErrorRaised`
- [ ] Implement an append-only event store (in-memory v0).
- [ ] Implement tool invocation record with:
  - timing
  - status codes
  - redaction markers
  - workspace before/after hash (or patch hash placeholder)
  - model/prompt provenance placeholders
- [ ] Implement minimal replay support:
  - rehydrate workspace from event stream (best-effort v0)

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/events.rs
crates/cocommand/src/events/event.rs
crates/cocommand/src/events/store.rs
crates/cocommand/src/events/redaction.rs
crates/cocommand/src/events/replay.rs
crates/cocommand/src/tools/invocation.rs
```

If a required change would affect files outside this list, stop and report.

---

## File Structure Constraints

- Use **file-based modules** (`module.rs` + `module/` directories).
- Do **not** use `mod.rs`.
- Public types must have brief doc comments.
- Keep dependencies minimal and platform-agnostic (no Tauri imports).

---

## Behavioral Requirements

- Event store preserves insertion order.
- Every tool execution can emit an invocation record.
- Redaction policy returns deterministic redacted views.
- Replay must be deterministic for the supported event set.

---

## Acceptance Criteria

- [ ] Event store append/read order works.
- [ ] Replay can rehydrate a workspace snapshot (limited v0 is fine).
- [ ] `cargo check --manifest-path crates/cocommand/Cargo.toml` passes.
- [ ] `cargo test --manifest-path crates/cocommand/Cargo.toml` passes.

---

## Definition of Done

- All tasks implemented.
- All tests in the checklist pass.
- No unrelated refactors or cleanups.
- No TODOs remain for this milestone.

---

## Test Checklist

- [ ] `cargo check --manifest-path crates/cocommand/Cargo.toml`
- [ ] `cargo test --manifest-path crates/cocommand/Cargo.toml`
- [ ] Event store append/read order tests
- [ ] Replay tests (empty → events → expected workspace state)

---

## Output Expectations

When finished, respond with:

1. Summary of changes
2. Files modified/created
3. Tests added or updated
4. Known limitations (if any)

---

## Failure Handling

If you encounter missing spec details or ambiguity, stop immediately and ask a clarification question.
