---
title: v0 Milestone 5 Handoff
status: completed
---

# AI Implementation Handoff — Core-5

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-5 — Command parsing v0 (tags + normalization)`**

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

- `apps/docs/src/content/docs/spec/01-terminology.md`
- `apps/docs/src/content/docs/spec/02-execution-model.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define a command parsing function that extracts:
  - raw text
  - normalized text (trim, collapse whitespace)
  - tagged app IDs (e.g., `@notes`)
- [ ] Support multiple tags in a single command.
- [ ] Return a structured `ParsedCommand` type.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/command.rs
crates/cocommand/src/command/parser.rs
crates/cocommand/src/command/types.rs
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

- Tag parsing is case-insensitive.
- Tags must be returned in order of appearance.
- If no tags are present, return an empty list.

---

## Acceptance Criteria

- [ ] `ParsedCommand` includes raw_text, normalized_text, and tags.
- [ ] `@notes @calendar create` returns tags `["notes", "calendar"]`.
- [ ] `hello world` returns no tags and normalized text.
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
- [ ] Parsing tests for tags, ordering, and normalization

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
