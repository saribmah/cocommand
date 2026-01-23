---
title: v0 Milestone 15 Handoff
status: completed
---

# AI Implementation Handoff — UI-3

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`UI-3 — Confirmation UI (permission flow)`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add minimal styling needed to display confirmations.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Add new backend routes or change core behavior.
- Implement UI-4+ (follow-up mode, recent actions, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-ui.md`

---

## Task List

Implement only the following tasks:

- [ ] Render confirmation panel when core returns `NeedsConfirmation`.
- [ ] Provide Confirm/Cancel actions.
- [ ] On confirm, call `confirm_action`.
- [ ] On cancel, clear pending confirmation.

---

## Allowed Files / Folders

You may create or modify only the following:

```text
apps/desktop/src/components/ConfirmPanel.tsx
apps/desktop/src/state/commandbar.ts
```

---

## Acceptance Criteria

- Destructive action triggers confirmation before execution.

---

## Definition of Done

- Confirmations are keyboard friendly (Enter/Esc).

---

## Tests

- Manual: “delete last note” requires confirm.
