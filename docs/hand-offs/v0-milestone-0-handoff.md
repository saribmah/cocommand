---
title: v0 Milestone 0 Handoff
---

# AI Implementation Handoff — Core-0

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-0 — Core crate skeleton & public API`**

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
- Implement later milestones (workspace, tools, events, routing, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/00-overview.md`
- `apps/docs/src/content/docs/spec/01-terminology.md`
- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/spec/03-workspace.md`
- `apps/docs/src/content/docs/spec/04-permissions.md`
- `apps/docs/src/content/docs/spec/05-routing.md`
- `apps/docs/src/content/docs/spec/06-extensions.md`
- `apps/docs/src/content/docs/spec/07-observability.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Create a minimal `Core` struct as the primary facade for desktop/UI integration.
- [ ] Define shared error and result types.
- [ ] Define stable method signatures on `Core` (stubs are fine):
  - `submit_command(text) -> CoreResponse`
  - `confirm_action(confirmation_id, decision) -> CoreResponse`
  - `get_workspace_snapshot() -> Workspace`
  - `get_recent_actions(limit) -> Vec<ActionSummary>`
- [ ] Create the module folder layout required for v0 (empty modules allowed).

Do not implement business logic; only structure and interfaces.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/lib.rs
crates/cocommand/src/core.rs
crates/cocommand/src/error.rs
crates/cocommand/src/types.rs
crates/cocommand/src/command/
crates/cocommand/src/routing/
crates/cocommand/src/planner/
crates/cocommand/src/workspace/
crates/cocommand/src/permissions/
crates/cocommand/src/tools/
crates/cocommand/src/events/
crates/cocommand/src/extensions/
crates/cocommand/src/builtins/
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

- `Core::new()` compiles and returns a valid instance.
- Public method signatures are stable and documented.
- No runtime logic is required in this milestone.

---

## Acceptance Criteria

- [ ] `cargo check -p cocommand` passes.
- [ ] `cargo test -p cocommand` passes with at least one placeholder test.
- [ ] Core crate compiles without any Tauri dependency.
- [ ] Module folders exist and are wired through `lib.rs`.

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
