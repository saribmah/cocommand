---
title: v0 Milestone 7 Handoff
status: completed
---

# AI Implementation Handoff — Core-7

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-7 — Planner v0 (plan structure + stub orchestration)`**

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
- Implement later milestones (built-ins, extensions, UI shell). 

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/spec/05-routing.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define a `Plan` structure that can hold ordered tool calls.
- [ ] Define a minimal `Planner` interface that takes a `ParsedCommand` and routing candidates.
- [ ] Implement a stub planner that returns a deterministic placeholder plan (no LLM integration yet).

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/planner.rs
crates/cocommand/src/planner/plan.rs
crates/cocommand/src/planner/planner.rs
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

- Planner output must be deterministic for the same input.
- Plan should preserve order of tool calls.

---

## Acceptance Criteria

- [ ] `Plan` exists and supports multiple tool calls.
- [ ] Stub planner returns a valid `Plan` for any input.
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
- [ ] Plan preserves ordering
- [ ] Planner stub returns deterministic output

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
