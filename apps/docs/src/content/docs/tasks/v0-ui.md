# tasks/v0-ui.md — Desktop UI / Shell Implementation Tasks (v0)

> **Scope:** `apps/desktop` (React UI) + `apps/desktop/src-tauri` wiring.
> **Non-scope:** Full settings UI, marketplace UI, fancy animations.

---

## UI-0 — Tauri bootstrap + Core wiring

### Tasks

* Create a single shared `Core` instance in Tauri state (`Arc<Core>`).
* Add invoke handlers:

    * `submit_command`
    * `confirm_action`
    * `get_workspace_snapshot`
    * `get_recent_actions`

### Targets

* `apps/desktop/src-tauri/src/main.rs`
* `apps/desktop/src-tauri/src/lib.rs`
* `apps/desktop/src-tauri/src/state.rs` (new)
* `apps/desktop/src-tauri/src/commands.rs` (new)

### Acceptance Criteria

* Frontend can call `submit_command` and get a response.

### Definition of Done

* Tauri layer contains no business logic beyond bridging.

### Tests

* Manual invoke from frontend (devtools) returns response

---

## UI-1 — Command Bar UI (idle + input + suggestions)

### Tasks

* Build the command bar component with:

    * input field
    * optional suggestion list (router candidates)
    * keyboard navigation
    * command submit on Enter
    * close on Esc
* Implement tagged `@app` pill rendering in input (optional v0; text is fine).

### Targets

* `apps/desktop/src/components/CommandBar.tsx`
* `apps/desktop/src/components/SuggestionList.tsx`
* `apps/desktop/src/state/commandbar.ts` (or Legend-State store)
* `apps/desktop/src/styles/`

### Acceptance Criteria

* User can open bar, type, see suggestions, submit.

### Definition of Done

* Bar is functional and stable; design polish minimal but consistent.

### Tests

* Manual: open/close, type, submit
* Keyboard nav works

---

## UI-2 — Result Rendering (artifact cards)

### Tasks

* Implement a normalized response format from core:

    * `type: "artifact" | "preview" | "confirmation" | "error"`
* Render artifact card UI:

    * title + body (markdown-ish ok)
    * actions (buttons)
* Support “Replace vs Stack” behavior (keep max 1–2 visible).

### Targets

* `apps/desktop/src/components/ResultCard.tsx`
* `apps/desktop/src/components/MarkdownView.tsx` (optional)
* `apps/desktop/src/types/core.ts`

### Acceptance Criteria

* “Summarize clipboard” shows an artifact card with buttons.

### Definition of Done

* Rendering covers core’s v0 response types.

### Tests

* Manual: see result after submit

---

## UI-3 — Confirmation UI (permission flow)

### Tasks

* Render confirmation panel when core returns `NeedsConfirmation`.
* Provide Confirm/Cancel actions.
* On confirm, call `confirm_action`.
* On cancel, clear pending confirmation.

### Targets

* `apps/desktop/src/components/ConfirmPanel.tsx`
* `apps/desktop/src/state/commandbar.ts`

### Acceptance Criteria

* Destructive action triggers confirmation before execution.

### Definition of Done

* Confirmations are keyboard friendly (Enter/Esc).

### Tests

* Manual: “delete last note” requires confirm

---

## UI-4 — Follow-up Mode UX

### Tasks

* When core indicates follow-up active, show a subtle UI cue:

    * placeholder “Refine the previous result…”
    * small badge “Follow-up”
* Ensure submitting continues same logical flow (core handles TTL).

### Targets

* `apps/desktop/src/components/CommandBar.tsx`
* `apps/desktop/src/state/commandbar.ts`

### Acceptance Criteria

* A second command submitted immediately feels continuous.

### Definition of Done

* Follow-up cue appears and disappears based on core state.

### Tests

* Manual: submit command → submit refinement within TTL

---

## UI-5 — Recent Actions panel (minimal observability UI)

### Tasks

* Add a lightweight “Recent Actions” view (can be a dropdown or separate panel):

    * list last N invocations
    * show tool name, status, duration
    * redact sensitive fields

### Targets

* `apps/desktop/src/components/RecentActions.tsx`
* `apps/desktop/src-tauri/src/commands.rs` (get_recent_actions)

### Acceptance Criteria

* User can inspect last actions and see success/failure.

### Definition of Done

* Works without leaking sensitive content.

### Tests

* Manual: actions show after executing tools

---

## UI-6 — Window behavior (hotkey + focus)

### Tasks

* Implement window show/hide toggle via hotkey (macOS).
* Ensure bar opens centered, focused input.
* Close on blur (optional), close on Esc.

### Targets

* `apps/desktop/src-tauri/src/window.rs`
* `crates/platform-macos/src/hotkeys.rs` (if needed)
* `apps/desktop/src-tauri/src/main.rs`

### Acceptance Criteria

* Hotkey reliably toggles command bar.

### Definition of Done

* No focus bugs; input always ready.

### Tests

* Manual: rapid toggle, multi-monitor basic test

---

## UI-7 — Desktop platform provider injection (macOS)

### Tasks

* Instantiate macOS platform providers (e.g. `MacClipboardProvider`) in the Tauri desktop bootstrap.
* Inject platform implementations into `Core` during initialization.
* Ensure platform providers are wrapped in `Arc<dyn Trait>` and live for the lifetime of the app.
* Keep all OS-specific wiring inside the desktop layer.

### Targets

```text
apps/desktop/src-tauri/src/main.rs
apps/desktop/src-tauri/src/state.rs
crates/platform-macos/src/clipboard.rs
```

### Acceptance Criteria

* Desktop app injects macOS platform providers into Core at startup.
* Clipboard built-in reads from the real system clipboard on macOS.
* Core remains platform-agnostic.

### Definition of Done

* Platform selection and wiring occur only in the desktop layer.
* Core initialization fails fast if required platform providers are missing.

### Tests

* Manual smoke test on macOS:

    * Copy text in another app
    * Open Cocommand
    * Run “show clipboard” or equivalent command
    * Copied text appears

---

# Appendix — UI Definition of Done (v0)

UI v0 is complete when:

* You can toggle the command bar with a hotkey.
* You can submit a command and see a result card.
* Destructive actions require confirmation.
* Follow-up commands work within TTL.
* Recent Actions shows last N invocations.
