---
title: v0 Milestone 9 Handoff
status: completed
---

# AI Implementation Handoff — Core-9

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-9 — Follow-up mode + session TTL v0 (core behavior)`**

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
- Implement future milestones (extension host, UI shell, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/spec/03-workspace.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Implement ephemeral follow-up context:
  - last result references
  - expires_at
  - turn limit
- [ ] Bias router during follow-up window.
- [ ] Add workspace mode transitions (`idle` ↔ `follow_up_active`).

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/core.rs
crates/cocommand/src/workspace/state.rs
crates/cocommand/src/routing/router.rs
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

- Follow-up context is short-lived and bounded.
- Router bias applies only while follow-up is active.
- After TTL expiry, follow-up behavior stops and clarification is required.

---

## Acceptance Criteria

- [ ] After creating a calendar event (or stub), “make it 2:30” resolves against the last entity reference.
- [ ] After TTL expiry, same input no longer resolves and triggers clarification.
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
- [ ] Follow-up within TTL works
- [ ] Follow-up after TTL does not

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
