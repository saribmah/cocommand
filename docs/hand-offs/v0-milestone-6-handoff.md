---
title: v0 Milestone 6 Handoff
status: completed
---

# AI Implementation Handoff — Core-6

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-6 — Routing v0 (metadata + shortlist + explainability)`**

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
- Implement later milestones (planner, built-ins, extensions, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/05-routing.md`
- `apps/docs/src/content/docs/spec/01-terminology.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define routing metadata types (keywords, examples, verbs, objects).
- [ ] Implement a simple router that:
  - uses keyword matching over examples/keywords
  - returns a bounded candidate set (e.g., top N)
  - returns an explanation string for why items were selected
- [ ] Support tag constraints: if tags are present, only return candidates from those apps.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/routing.rs
crates/cocommand/src/routing/metadata.rs
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

- Routing must be deterministic for the same input.
- Tag constraints must be enforced as a hard allowlist.
- Router returns an explanation per candidate (simple string is fine).

---

## Acceptance Criteria

- [ ] A command with tags only returns tagged apps.
- [ ] A command with no tags returns a bounded candidate list.
- [ ] Router returns explanation strings for each candidate.
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
- [ ] Routing returns deterministic ordering
- [ ] Tag constraint tests
- [ ] Explanation string exists for each candidate

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
