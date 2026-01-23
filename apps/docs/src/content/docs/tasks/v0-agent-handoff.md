Perfect. This is the missing piece that makes the whole “docs → AI builds the app” workflow actually work.

Below is a **drop-in `tasks/v0-agent-handoff.md`**. This is a **prompt template**, not code. You’ll reuse it for *each* milestone by filling in the bracketed sections.

This template is intentionally strict. It prevents:

* overbuilding
* touching random files
* inventing APIs
* drifting from your spec

---

# tasks/v0-agent-handoff.md

**AI Implementation Handoff Template (v0)**

> **Purpose:**
> This document is used to hand off a single implementation milestone to an AI coding agent.
> The agent must follow the instructions in this file exactly.

---

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing **Milestone:**

> **`[MILESTONE_ID] – [MILESTONE_TITLE]`**

This milestone is part of **v0** and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

* Create or modify files listed under **Allowed Files**.
* Implement only the tasks explicitly listed in **Task List**.
* Add tests only for the behavior described in **Acceptance Criteria**.
* Use existing abstractions as defined in the docs.

### You MUST NOT:

* Modify files outside **Allowed Files**.
* Introduce new modules, crates, or services.
* Change public APIs unless explicitly instructed.
* Add UI polish, animations, or speculative features.
* Skip tests listed in **Test Checklist**.

If something is missing or unclear, **stop and ask for clarification**.

---

## References (READ FIRST)

You must read and follow:

* `docs/terminology.md`
* `docs/execution-model.md`
* `docs/workspace.md`
* `docs/permissions.md`
* `docs/routing.md`
* `docs/extensions.md` (if applicable)
* `docs/observability.md`
* `tasks/v0.md`
* `tasks/v0-core.md` or `tasks/v0-ui.md` (as applicable)

These documents are the **single source of truth**.

---

## Task List

Implement **only** the following tasks:

* [ ] `[TASK_1]`
* [ ] `[TASK_2]`
* [ ] `[TASK_3]`
* [ ] `[TASK_N]`

Each task must be completed fully before moving to the next.

---

## Allowed Files / Folders

You may create or modify **only** the following:

```text
[PATH_1]
[PATH_2]
[PATH_3]
```

If a required change would affect files outside this list, **stop and report**.

---

## File Structure Constraints

* Use **file-based modules** (`module.rs` + `module/` directories).
* Do **not** use `mod.rs`.
* Follow existing naming conventions.
* Public types must be documented with doc comments.

---

## Behavioral Requirements

The implementation **must** satisfy:

* `[BEHAVIOR_RULE_1]`
* `[BEHAVIOR_RULE_2]`
* `[BEHAVIOR_RULE_3]`

These rules come directly from the spec and are non-negotiable.

---

## Acceptance Criteria

The milestone is considered complete **only if**:

* [ ] `[ACCEPTANCE_CRITERION_1]`
* [ ] `[ACCEPTANCE_CRITERION_2]`
* [ ] `[ACCEPTANCE_CRITERION_3]`

If any criterion cannot be met, explain why and stop.

---

## Definition of Done

All of the following must be true:

* All tasks are implemented.
* All tests in the checklist pass.
* Code compiles without warnings.
* No unrelated refactors or cleanups were performed.
* No TODOs remain for this milestone.

---

## Test Checklist

You must run and pass:

* [ ] `cargo check -p cocommand`
* [ ] `cargo test -p cocommand`
* [ ] `[ADDITIONAL_TESTS_IF_ANY]`

If tests are missing, you must add them.

---

## Output Expectations

When finished, respond with:

1. **Summary of changes** (bullet points)
2. **Files modified/created**
3. **Tests added or updated**
4. **Known limitations (if any)**

Do **not** include code explanations unless requested.

---

## Failure Handling

If you encounter:

* missing spec details,
* ambiguous requirements,
* conflicting instructions,

**Stop immediately** and ask a clarification question before continuing.

---

## Reminder

> **This is a deterministic system.**
> Do not rely on “LLM intuition.”
> Follow the spec exactly.

---

### Example Usage (DO NOT IMPLEMENT)

```
Milestone: Core-3 — Tool system v0
Allowed Files:
- crates/cocommand/src/tools.rs
- crates/cocommand/src/tools/registry.rs
- crates/cocommand/src/tools/executor.rs
...
```

---

## Why this template matters

This template ensures:

* AI implementations are predictable
* milestones don’t bleed into each other
* your architecture stays intact
* you can safely parallelize work across agents

---

If you want, next we can:

* generate **pre-filled handoff files** for the first 2–3 milestones (so you can literally copy-paste and run),
* or create a **GitHub issue template** version of this that mirrors the same constraints.
