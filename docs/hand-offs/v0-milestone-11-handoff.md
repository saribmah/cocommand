---
title: v0 Milestone 11 Handoff
status: completed
---

# AI Implementation Handoff — Core-11

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-11 — Platform abstraction & core integration v0`**

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
- Introduce OS-specific dependencies in the core crate.
- Add UI logic or Tauri dependencies.
- Implement future milestones (platform implementations, desktop integration, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Define platform abstraction traits in the core crate (e.g. `ClipboardProvider`).
- [ ] Update built-in Clipboard application to depend **only on the platform trait**, never on OS-specific APIs.
- [ ] Provide a `NullClipboardProvider` or `MockClipboardProvider` for tests and non-desktop environments.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/platform.rs
crates/cocommand/src/builtins/clipboard.rs
```

---

## Acceptance Criteria

- Core crate compiles and runs without any platform-specific dependencies.
- Clipboard built-in reads clipboard content via the platform trait.
- Unit tests use mock implementations only.

---

## Definition of Done

- Core depends exclusively on platform traits.
- No macOS (or other OS) imports exist in `crates/cocommand`.

---

## Tests

- Unit test using `MockClipboardProvider` returns expected clipboard text.
