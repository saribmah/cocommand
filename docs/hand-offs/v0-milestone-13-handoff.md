---
title: v0 Milestone 13 Handoff
status: completed
---

# AI Implementation Handoff — UI-1

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`UI-1 — Command Bar UI (idle + input + suggestions)`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add minimal styling to make the command bar usable.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Add new backend routes or change core behavior.
- Implement UI-2+ (result cards, confirmation, follow-up mode).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-ui.md`

---

## Task List

Implement only the following tasks:

- [ ] Build the command bar component with:
  - input field
  - optional suggestion list (router candidates)
  - keyboard navigation
  - command submit on Enter
  - close on Esc
- [ ] Implement tagged `@app` pill rendering in input (optional v0; text is fine).

---

## Allowed Files / Folders

You may create or modify only the following:

```text
apps/desktop/src/components/CommandBar.tsx
apps/desktop/src/components/SuggestionList.tsx
apps/desktop/src/state/commandbar.ts
apps/desktop/src/styles/
```

---

## Acceptance Criteria

- User can open bar, type, see suggestions, submit.

---

## Definition of Done

- Bar is functional and stable; design polish minimal but consistent.

---

## Tests

- Manual: open/close, type, submit.
- Keyboard nav works.
