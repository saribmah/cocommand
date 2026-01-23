---
title: v0 Milestone 3 Handoff
status: completed
---

# AI Implementation Handoff — Core-3

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-3 — Tool system v0 (registry + schema validation + executor)`**

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
- Implement later milestones (permissions, routing, planner, built-ins, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/spec/03-workspace.md`
- `apps/docs/src/content/docs/spec/04-permissions.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define `Tool` abstraction:
  - `tool_id`
  - input/output JSON schema
  - risk level
  - handler signature
- [ ] Implement `ToolRegistry`:
  - kernel tools always mounted
  - instance-mounted tools by instance_id
- [ ] Implement tool executor pipeline:
  - validate args against schema
  - permissions gate stub (no real enforcement yet)
  - execute handler
  - record events + invocation record
  - apply workspace patch (if kernel tool)

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/tools.rs
crates/cocommand/src/tools/registry.rs
crates/cocommand/src/tools/schema.rs
crates/cocommand/src/tools/executor.rs
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

- Tool registry must return kernel tools and per-instance tools deterministically.
- Schema validation must run before handler execution.
- Executor must emit event(s) and an invocation record for each call.

---

## Acceptance Criteria

- [ ] Tools can be registered and invoked with JSON args.
- [ ] Invalid args fail before execution.
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
- [ ] Registry lookup tests
- [ ] Schema validation tests
- [ ] Executor emits events + invocation record tests

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
