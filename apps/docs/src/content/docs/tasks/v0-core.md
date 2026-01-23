# tasks/v0-core.md — Core / Backend Implementation Tasks (v0)

> **Scope:** `crates/cocommand`, `crates/platform-macos`, Deno extension host + protocol.
> **Non-scope:** React UI polish beyond basic command bar, cards, confirmations.

---

## Core-0 — Core crate skeleton & public API

### Tasks

* Create `Core` struct as the primary facade for desktop/UI.
* Define shared error and result types.
* Define “core orchestration” methods:

    * `submit_command(text) -> CoreResponse`
    * `confirm_action(confirmation_id, decision) -> CoreResponse`
    * `get_workspace_snapshot() -> Workspace`
    * `get_recent_actions(limit) -> Vec<ActionSummary>`

### Targets

* `crates/cocommand/src/lib.rs`
* `crates/cocommand/src/core.rs`
* `crates/cocommand/src/error.rs`
* `crates/cocommand/src/types.rs`

### Acceptance Criteria

* Core compiles and has stable method signatures.
* Core crate has **no dependency on Tauri**.

### Definition of Done

* `Core` can be constructed and returns placeholder responses.

### Tests

* `cargo test -p cocommand` basic initialization test.

---

## Core-1 — Workspace v0 (schema + invariants + kernel tools)

### Tasks

* Implement Workspace structs (schema v0).
* Implement invariants check helper.
* Implement Kernel Tools as pure Rust functions mutating workspace:

    * open/close/focus/mount/unmount
* Implement atomic patch application:

    * either mutation commits fully or not at all

### Targets

* `crates/cocommand/src/workspace.rs`
* `crates/cocommand/src/workspace/state.rs`
* `crates/cocommand/src/workspace/invariants.rs`
* `crates/cocommand/src/workspace/kernel_tools.rs`
* `crates/cocommand/src/workspace/patch.rs`

### Acceptance Criteria

* Workspace serializes/deserializes cleanly.
* Invalid states are rejected by invariants.

### Definition of Done

* Kernel tools are the only mutation entry points.

### Tests

* Unit tests for invariants and kernel tools.

---

## Core-2 — Events & Observability v0 (event stream + invocation records)

### Tasks

* Define canonical event model and event store.
* Implement invocation record:

    * timing, status, error code, redaction marker
    * workspace before/after hashes or patch hash
    * runtime provenance placeholders
* Add minimal replay support:

    * rehydrate workspace from checkpoint + event tail (checkpoint optional v0)

### Targets

* `crates/cocommand/src/events.rs`
* `crates/cocommand/src/events/event.rs`
* `crates/cocommand/src/events/store.rs`
* `crates/cocommand/src/events/redaction.rs`
* `crates/cocommand/src/events/replay.rs`
* `crates/cocommand/src/tools/invocation.rs`

### Acceptance Criteria

* Every tool execution emits consistent events.
* Redaction policy returns deterministic “redacted view”.

### Definition of Done

* Can show “Recent Actions” from events without UI.

### Tests

* Event ordering, replay correctness tests.

---

## Core-3 — Tool system v0 (registry + schema validation + executor)

### Tasks

* Implement Tool abstraction:

    * IDs, schemas, risk, handler signature
* Implement ToolRegistry:

    * kernel tools always mounted
    * instance mounted tools
* Implement tool executor pipeline:

    * validate args against schema
    * enforce permissions (hook into Core-4)
    * call handler
    * record events and invocation record
    * apply workspace patch (kernel tools)

### Targets

* `crates/cocommand/src/tools.rs`
* `crates/cocommand/src/tools/registry.rs`
* `crates/cocommand/src/tools/schema.rs`
* `crates/cocommand/src/tools/executor.rs`

### Acceptance Criteria

* Tools can be invoked deterministically with JSON args.
* Invalid args fail before execution.

### Definition of Done

* `Core` uses executor for all tool calls.

### Tests

* Registry lookup tests
* Schema validation tests
* Executor emits events tests

---

## Core-4 — Permissions v0 (scopes + risk + enforcement + confirmation)

### Tasks

* Implement permission store (allow/ask/deny).
* Implement risk levels: safe/confirm/destructive.
* Implement enforcement function that returns:

    * Allowed
    * Denied
    * NeedsConfirmation(confirmation_id)
* Integrate into executor pipeline.
* Add workspace mode transition for confirmation pending.

### Targets

* `crates/cocommand/src/permissions.rs`
* `crates/cocommand/src/permissions/scopes.rs`
* `crates/cocommand/src/permissions/risk.rs`
* `crates/cocommand/src/permissions/store.rs`
* `crates/cocommand/src/permissions/enforcement.rs`

### Acceptance Criteria

* Destructive tools require confirmation round-trip.
* Denied actions do not mutate workspace.

### Definition of Done

* Confirmation IDs are stable and logged as events.

### Tests

* Safe tools run
* Destructive tools block until confirmed
* Denied tools fail + events emitted

---

## Core-5 — Command parsing v0 (tags + normalization)

### Tasks

* Parse raw user text into `CommandRequest`.
* Extract `@app` tags as **allowlist**.
* Normalize command text (strip tags, trim, preserve quoted strings).
* Handle edge cases (`@` inside emails, @ inside code blocks).

### Targets

* `crates/cocommand/src/command.rs`
* `crates/cocommand/src/command/parser.rs`
* `crates/cocommand/src/command/tagging.rs`
* `crates/cocommand/src/command/types.rs`

### Acceptance Criteria

* Tags are extracted accurately and deterministically.

### Definition of Done

* Parser is used by `Core::submit_command`.

### Tests

* Tag parsing tests
* Normalization tests

---

## Core-6 — Routing v0 (capability router)

### Tasks

* Define routing metadata model:

    * app keywords/examples
    * capability keywords/examples (optional v0)
* Implement router:

    * lexical scoring (keywords/examples)
    * workspace priors (recent/pinned/open/focused)
    * tag allowlist filter
    * bounded output (<= 7 apps)
* Store last routing candidates in workspace context.

### Targets

* `crates/cocommand/src/routing.rs`
* `crates/cocommand/src/routing/metadata.rs`
* `crates/cocommand/src/routing/router.rs`
* `crates/cocommand/src/routing/scoring.rs`

### Acceptance Criteria

* Router returns a small, relevant candidate set.
* Tagged commands restrict candidates strictly.

### Definition of Done

* `Core` routes before planning.

### Tests

* Router candidate correctness tests
* Tag allowlist tests

---

## Core-7 — Planner v0 (stub planner + interface)

### Tasks

* Define planner interface returning:

    * proposed tool calls (sequence)
    * clarification request
* Implement stub planner (rule-based) for built-ins:

    * “show last note”
    * “summarize clipboard”
    * “calculate …”
* Add placeholders for real LLM providers (optional v0).
* Record planner provenance fields in events (hash/version).

### Targets

* `crates/cocommand/src/planner.rs`
* `crates/cocommand/src/planner/types.rs`
* `crates/cocommand/src/planner/stub.rs`

### Acceptance Criteria

* End-to-end works with stub planner.

### Definition of Done

* Real LLM can be added without changing Core API.

### Tests

* Stub planner outputs expected tool calls

---

## Core-8 — Built-ins v0 (Clipboard, Notes, Calculator + “preview output”)

### Tasks

* Implement Built-in application definitions:

    * metadata
    * capabilities/tools
    * routing metadata
    * risk levels
* Clipboard:

    * read latest
    * list history (v0 can be limited)
* Notes:

    * list, latest, create, update, delete (delete destructive)
    * return “preview payload” for show commands
* Calculator:

    * parse/eval simple expressions safely

### Targets

* `crates/cocommand/src/builtins.rs`
* `crates/cocommand/src/builtins/clipboard.rs`
* `crates/cocommand/src/builtins/notes.rs`
* `crates/cocommand/src/builtins/calculator.rs`

### Acceptance Criteria

* Core can route→plan→execute built-in commands end-to-end.

### Definition of Done

* Built-ins register tools and routing metadata at core startup.

### Tests

* End-to-end tests for each built-in command flow
* Confirm flow for notes.delete

---

## Core-9 — Follow-up mode v0 (bounded continuity)

### Tasks

* Track last result references (entity IDs) and expiry.
* Add follow-up TTL and turn counter.
* Bias router during follow-up window.
* Support “modify last entity” flows for at least one built-in (Notes update).

### Targets

* `crates/cocommand/src/core.rs`
* `crates/cocommand/src/workspace/state.rs`
* `crates/cocommand/src/routing/router.rs`

### Acceptance Criteria

* “show last note” then “copy it” works within TTL.
* After TTL, “copy it” triggers clarification.

### Definition of Done

* Follow-up is deterministic, bounded, and logged.

### Tests

* TTL expiry tests
* Follow-up routing bias tests

---

## Core-10 — Deno extension host v0 (manifest + RPC + invoke)

### Tasks

* Build Deno host process:

    * load extension
    * list tools
    * invoke tool
* Define JSON-RPC protocol:

    * `loadExtension`, `listTools`, `invokeTool`, `unloadExtension`
* Implement Rust-side extension manager:

    * parse manifest
    * start/monitor host process
    * invoke tools via RPC
    * ingest routing metadata

### Targets

* `crates/cocommand/src/extensions.rs`
* `crates/cocommand/src/extensions/manifest.rs`
* `crates/cocommand/src/extensions/rpc.rs`
* `crates/cocommand/src/extensions/lifecycle.rs`
* `apps/extension-host/main.ts` (new)
* `apps/extension-host/protocol.ts` (new)
* `apps/extension-host/loader.ts` (new)

### Acceptance Criteria

* An extension tool can be executed end-to-end and logged.

### Definition of Done

* Timeouts + crashes are handled gracefully.

### Tests

* Integration test spawning Deno host
* Timeout test (host hangs)

---

## Core-11 — Platform abstraction & core integration v0

### Tasks

* Define platform abstraction traits in the core crate (e.g. `ClipboardProvider`).
* Update built-in Clipboard application to depend **only on the platform trait**, never on OS-specific APIs.
* Provide a `NullClipboardProvider` or `MockClipboardProvider` for tests and non-desktop environments.

### Targets

```text
crates/cocommand/src/platform.rs
crates/cocommand/src/builtins/clipboard.rs
```

### Acceptance Criteria

* Core crate compiles and runs without any platform-specific dependencies.
* Clipboard built-in reads clipboard content via the platform trait.
* Unit tests use mock implementations only.

### Definition of Done

* Core depends exclusively on platform traits.
* No macOS (or other OS) imports exist in `crates/cocommand`.

### Tests

* Unit test using `MockClipboardProvider` returns expected clipboard text.

---
