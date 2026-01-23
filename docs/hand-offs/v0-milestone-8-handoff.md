---
title: v0 Milestone 8 Handoff
status: completed
---

# AI Implementation Handoff — Core-8

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-8 — Built-in apps v0 (Clipboard, Notes, Calculator)`**

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
- Implement future milestones (extensions, UI shell, follow-up mode, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/01-terminology.md`
- `apps/docs/src/content/docs/spec/04-permissions.md`
- `apps/docs/src/content/docs/spec/05-routing.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Implement built-in Application definitions for Clipboard, Notes, and Calculator, including:
  - app metadata
  - capabilities / tool definitions
  - routing metadata
  - permission scopes + risk levels
- [ ] Implement tool handlers:
  - Clipboard: `list`, `latest`
  - Notes: `list`, `latest`, `create`, `update`, `delete` (delete = destructive)
  - Calculator: `eval`, `parse` (safe)
- [ ] Register built-ins into ToolRegistry and Router.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/builtins.rs
crates/cocommand/src/builtins/clipboard.rs
crates/cocommand/src/builtins/notes.rs
crates/cocommand/src/builtins/calculator.rs
crates/cocommand/src/tools/registry.rs
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

- Notes delete must require confirmation (risk level = destructive).
- Built-ins must register their routing metadata and tools at startup.
- Calculator tools are safe and require no confirmation.

---

## Acceptance Criteria

- [ ] Commands can be routed → planned → executed end-to-end using built-ins.
- [ ] Notes delete returns `NeedsConfirmation` before execution.
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
- [ ] End-to-end test harness: "show last note" returns payload
- [ ] Confirmation flow: "delete last note" requires confirmation

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
