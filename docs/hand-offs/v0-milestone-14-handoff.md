---
title: v0 Milestone 14 Handoff
status: pending
---

# AI Implementation Handoff — UI-2

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`UI-2 — Result Rendering (artifact cards)`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add minimal styling needed to render artifact cards.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Add new backend routes or change core behavior.
- Implement UI-3+ (confirmation panel, follow-up mode, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-ui.md`

---

## Task List

Implement only the following tasks:

- [ ] Implement a normalized response format from core:
  - `type: "artifact" | "preview" | "confirmation" | "error"`
- [ ] Render artifact card UI:
  - title + body (markdown-ish ok)
  - actions (buttons)
- [ ] Support “Replace vs Stack” behavior (keep max 1–2 visible).

---

## Allowed Files / Folders

You may create or modify only the following:

```text
apps/desktop/src/components/ResultCard.tsx
apps/desktop/src/components/MarkdownView.tsx
apps/desktop/src/types/core.ts
```

---

## Acceptance Criteria

- “Summarize clipboard” shows an artifact card with buttons.

---

## Definition of Done

- Rendering covers core’s v0 response types.

---

## Tests

- Manual: see result after submit.
