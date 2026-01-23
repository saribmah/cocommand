---
title: v0 Milestone 4 Handoff
status: completed
---

# AI Implementation Handoff — Core-4

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-4 — Permissions v0 (scopes + risk + enforcement + confirmation)`**

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
- Implement later milestones (routing, planner, built-ins, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/04-permissions.md`
- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define permission scopes and risk levels (safe/confirm/destructive).
- [ ] Implement permission decision store (allow/ask/deny).
- [ ] Implement enforcement function that returns:
  - Allowed
  - Denied
  - NeedsConfirmation(confirmation_id)
- [ ] Integrate enforcement hook into the tool executor pipeline (stub is fine but must be wired).
- [ ] Add workspace mode transition to `AwaitingConfirmation` when confirmation is required.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/permissions.rs
crates/cocommand/src/permissions/scopes.rs
crates/cocommand/src/permissions/risk.rs
crates/cocommand/src/permissions/store.rs
crates/cocommand/src/permissions/enforcement.rs
crates/cocommand/src/tools/executor.rs
crates/cocommand/src/workspace/state.rs
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

- Destructive tools require confirmation round-trip.
- Denied actions do not mutate workspace or emit success events.
- Confirmation IDs are stable and recorded as events.

---

## Acceptance Criteria

- [ ] Safe tools execute without confirmation.
- [ ] Destructive tools return a confirmation requirement.
- [ ] Denied tools return a denied outcome and do not mutate workspace.
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
- [ ] Safe tool executes without confirmation
- [ ] Destructive tool yields confirmation required
- [ ] Denied tool returns error and no state change

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
