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

## Core-3A — Tool Assembly & Mounting Policy (runtime-managed)

### Tasks

* Define the v0 tool assembly policy:

    * Kernel tools are always available.
    * Application tools become available only when the runtime marks an application instance as **active** and mounts its tools.
    * Tool mounting is **runtime-managed** (system-owned) and must not be required as an LLM action.
* Implement a runtime helper API (pick one and document it):

    * **Option A:** `ensure_application_ready(app_id, ui, focus) -> instance_id`
      Opens instance if needed and mounts its tools in one operation.
    * **Option B:** After executing `open_application`, the runtime automatically calls `mount_application_tools(instance_id)` (internal).
* Implement budget enforcement for mounts:

    * Define `max_mounted_apps` and `max_mounted_tools_per_app` (v0 defaults are OK).
    * If mounting would exceed budget, unmount tools from least-recently-used inactive instances first.
* Ensure mounts are **derived / recomputable** and do not need to be persisted.

### Targets

```text
crates/cocommand/src/workspace/kernel_tools.rs
crates/cocommand/src/tools/registry.rs
crates/cocommand/src/tools/executor.rs
crates/cocommand/src/core.rs
```

### Acceptance Criteria

* The LLM/planner does not need to explicitly call a `mount_application_tools` tool to use application tools.
* After the runtime “ensures” an app instance is ready, tools for that instance can be executed successfully.
* Mount budgets are enforced deterministically.

### Definition of Done

* There is a single, documented place in the runtime responsible for mounting/unmounting tools.
* Mounting behavior is consistent across built-ins and extensions.

### Test Checklist

* Unit test: `ensure_application_ready(notes)` results in tools mounted for that instance (registry has those tools).
* Unit test: mounting beyond budget evicts older mounts deterministically.

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

## Core-7A — Execution Orchestration Loop (route → plan → ensure → execute)

### Tasks

* Implement the core orchestration loop for `Core::submit_command(text)`:

    1. Parse command and tags (`CommandRequest`)
    2. Route to candidates (`RouterOutput`)
    3. Plan to actions (`Plan` with proposed steps)
    4. For each step requiring an app tool:

        * ensure required app(s) are ready (open instance if needed + mount tools per Core-3A)
    5. Execute tool calls via executor
    6. Record events + update workspace (including follow-up references)
* Implement preview vs open behavior:

    * Read-only commands (e.g., “show last note”) return a `CoreResponse::Preview` and must not create an application instance.
    * Commands requiring persistent interaction (e.g., “open notes”, “edit note”) create an application instance and may set focus.
* Define how “open app” in a plan is handled:

    * If plan includes `open_application`, runtime executes it and then mounts tools automatically (internal policy).
    * Alternatively, planner never emits raw open; it emits “use app X” and runtime ensures it.
* Implement a v0 “two-pass” behavior only if needed:

    * Pass 1: plan selects apps/capabilities
    * Runtime ensures mounts
    * Pass 2: plan emits specific tool calls (optional; stub planner may not need)

### Targets

```text
crates/cocommand/src/core.rs
crates/cocommand/src/command/parser.rs
crates/cocommand/src/routing/router.rs
crates/cocommand/src/planner/stub.rs
crates/cocommand/src/tools/executor.rs
crates/cocommand/src/events/store.rs
crates/cocommand/src/workspace/patch.rs
```

### Acceptance Criteria

* `Core::submit_command` can run end-to-end without the UI knowing about tools/mounting.
* Multi-step commands can open/mount apps as required without LLM micromanagement.

### Definition of Done

* The runtime (Core) is the single orchestrator of:

    * routing output
    * planner output
    * tool execution
    * mounting/unmounting decisions
    * event recording

### Test Checklist

* End-to-end unit test using stub planner:

    * “edit last note” triggers ensure(notes) then notes.update tool call
* Test that tools are not callable before ensure/mount

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

## Core-12 — Desktop Bridge Contract (Core API + Response Types)

### Tasks

* Define stable request/response types used by the desktop UI:

    * `SubmitCommandRequest { text: String }`
    * `CoreResponse` (see response variants below)
    * `ConfirmActionRequest { confirmation_id: String, decision: bool }`
    * `ActionSummary` for Recent Actions UI
* Implement `Core::submit_command(text)` and `Core::confirm_action(...)` to return **only** `CoreResponse`.
* Ensure responses are fully serializable (serde) and contain no non-serializable types.

### CoreResponse (v0) — Required Variants

* `Artifact`: a shell-renderable result with optional actions
* `Preview`: read-only preview payload (e.g., last note)
* `Confirmation`: confirmation prompt payload (risk actions)
* `Error`: user-displayable error payload

### Targets

```text
crates/cocommand/src/core.rs
crates/cocommand/src/types.rs
crates/cocommand/src/error.rs
```

### Acceptance Criteria

* Core produces responses in a single stable shape for all command outcomes.
* Core responses are serializable via `serde` without custom hacks.
* Confirmation responses include a stable `confirmation_id` usable by UI.

### Definition of Done

* Desktop layer does not need to understand tools, workspace internals, or events to render results.
* CoreResponse is the only format used across the Tauri boundary.

### Test Checklist

* Unit test: `serde_json::to_string(&CoreResponse)` succeeds for each variant.
* Unit test: stub command produces each variant at least once.

---
