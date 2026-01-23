---
title: v0 Milestone 1 Handoff
status: completed
---

# AI Implementation Handoff — Core-1

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-1 — Workspace v0 (schema + invariants + kernel tools)`**

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
- Implement later milestones (events, tools, routing, permissions, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/01-terminology.md`
- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/spec/03-workspace.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Implement Workspace structs matching the Workspace Schema v0.
- [ ] Implement invariant checks (focus validity, mounted tools referencing open instances, etc.).
- [ ] Implement Kernel Tools as pure Rust functions:
  - `open_application`
  - `close_application`
  - `focus_application`
  - `mount_tools`
  - `unmount_tools`
- [ ] Implement atomic mutation application (apply patch/transactional updates).

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/workspace.rs
crates/cocommand/src/workspace/state.rs
crates/cocommand/src/workspace/invariants.rs
crates/cocommand/src/workspace/kernel_tools.rs
crates/cocommand/src/workspace/patch.rs
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

- Workspace is the single source of truth for open instances, focus, mounts, context, policy, and journal refs.
- Kernel tools are the only mutation entry points.
- Invariant violations return structured errors (do not panic).
- Kernel tools are idempotent where specified.

---

## Acceptance Criteria

- [ ] Workspace can be created, serialized, and deserialized.
- [ ] Kernel tools mutate workspace only through sanctioned functions.
- [ ] Invariant violations are detected and returned as errors.
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
- [ ] Unit tests for invariants (focus points to open instance, mounted tools require active instance)
- [ ] Unit tests for kernel tool behaviors (open/focus/close/mount/unmount)

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
