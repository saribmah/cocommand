---
title: v0 Milestone 18 Handoff
status: completed
---

# AI Implementation Handoff — UI-4

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`UI-4 — Follow-up Mode UX`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add minimal UI cues to indicate follow-up state.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Add new backend routes or change core behavior.
- Implement UI-5+ (recent actions, window behavior, platform provider injection, etc.).

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-ui.md`

---

## Task List

Implement only the following tasks:

- [ ] When core indicates follow-up active, show a subtle UI cue:
  - placeholder “Refine the previous result…”
  - small badge “Follow-up”
- [ ] Ensure submitting continues same logical flow (core handles TTL).

---

## Allowed Files / Folders

You may create or modify only the following:

```text
apps/desktop/src/components/CommandBar.tsx
apps/desktop/src/state/commandbar.ts
```

---

## Acceptance Criteria

- A second command submitted immediately feels continuous.

---

## Definition of Done

- Follow-up cue appears and disappears based on core state.

---

## Tests

- Manual: submit command → submit refinement within TTL.
