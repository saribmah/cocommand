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

## UI-0A — Core Bridge Integration (Tauri invoke contract)

### Tasks

* Implement Tauri invoke handlers that exactly mirror the Core bridge contract:

    * `submit_command(text) -> CoreResponse`
    * `confirm_action(confirmation_id, decision) -> CoreResponse`
    * `get_recent_actions(limit) -> Vec<ActionSummary>`
    * `get_workspace_snapshot() -> Workspace` (optional for v0 UI, but useful for debugging)
* Ensure the frontend uses these invoke calls as the only backend integration path.

### Targets

```text
apps/desktop/src-tauri/src/commands.rs
apps/desktop/src-tauri/src/state.rs
apps/desktop/src/types/core.ts
```

### Acceptance Criteria

* Frontend can submit a command and receive a `CoreResponse` without shape mismatches.
* Confirmation response round-trip works (UI → confirm_action → updated response).

### Definition of Done

* UI and Core share a stable boundary: no tool IDs leak into UI.
* All backend calls are routed through the Tauri commands layer.

### Test Checklist

* Manual: run a command and render Artifact
* Manual: run destructive action and confirm

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

## **UI-8 — End-to-End Wiring (UI State Machine + CoreResponse Rendering)**

> This is the explicit “gel together” milestone you’re asking for.

### Tasks

* Implement a minimal UI state machine driven exclusively by `CoreResponse`:

    * `idle` → `executing` → (`artifact` | `preview` | `confirmation` | `error`)
* On Enter:

    * call `submit_command(text)`
    * render response
* On Confirm:

    * call `confirm_action(confirmation_id, decision=true)`
    * render updated response
* On Cancel:

    * clear confirmation UI and return to idle
* Implement “Follow-up cue” purely based on fields returned by core (e.g., `follow_up_active: bool` in response metadata) or by querying workspace snapshot (debug mode).

### Targets

```text
apps/desktop/src/components/CommandBar.tsx
apps/desktop/src/components/ResultCard.tsx
apps/desktop/src/components/ConfirmPanel.tsx
apps/desktop/src/state/commandbar.ts
apps/desktop/src/types/core.ts
```

### Acceptance Criteria

* Running a safe command shows an Artifact result.
* Running a preview command shows a Preview result.
* Running a destructive command shows Confirmation, and confirming executes and returns Artifact/Preview.
* The UI never calls tools directly; all actions go through core.

### Behavioral Requirement

* UI must not attempt to manage tool mounting/opening logic. The UI calls `submit_command` and renders the returned `CoreResponse`; the core runtime handles routing, planning, instance lifecycle, and tool availability.

### Definition of Done

* UI is fully driven by the `CoreResponse` contract (no hidden coupling).
* At least 3 sample flows work end-to-end:

    1. Calculator command → Artifact
    2. Show last note → Preview
    3. Delete last note → Confirmation → Artifact/Preview

### Test Checklist

* Manual smoke checklist for the 3 flows above
* Optional: component-level tests for response rendering

---

# Appendix — UI Definition of Done (v0)

UI v0 is complete when:

* You can toggle the command bar with a hotkey.
* You can submit a command and see a result card.
* Destructive actions require confirmation.
* Follow-up commands work within TTL.
* Recent Actions shows last N invocations.
