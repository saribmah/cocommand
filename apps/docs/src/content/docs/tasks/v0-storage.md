# v0 Storage Implementation Plan

This plan introduces a storage module with a facade trait and specialized sub-stores. It is designed to fit the current codebase (Core + built-ins) and allow swapping providers (memory, SQLite, filesystem) without changing core logic.

## Goals

- Keep core logic storage-agnostic via traits.
- Support deterministic event logging and recent actions.
- Add domain stores (clipboard, settings, snapshots) without a monolithic CRUD interface.
- Start with memory-only v0, but keep SQLite-ready interfaces.

---

## Phase 0 — Inventory + Targets

Current touch points:

- `crates/cocommand/src/core.rs` (needs storage injection)
- `crates/cocommand/src/events/store.rs` (in-memory only)
- `crates/cocommand/src/types.rs` (ActionSummary shape incomplete)
- `crates/cocommand/src/builtins/clipboard.rs` (history storage missing)
- `apps/desktop/src-tauri/src/state.rs` (Core::new signature changes)

---

## Phase 1 — Add Storage Module (traits + memory impl)

Tasks

- Create `crates/cocommand/src/storage.rs` with:
  - Specialized traits: `EventLog`, `SnapshotStore`, `KvStore`, `ClipboardStore`
  - Facade trait: `Storage`
  - Domain structs: `EventRecord`, `WorkspaceSnapshot`, `ClipboardEntry`
- Implement `MemoryStorage` that satisfies `Storage`.
- Export `storage` in `crates/cocommand/src/lib.rs`.

Acceptance Criteria

- Memory storage compiles and is testable in isolation.
- No core logic changes yet.

---

## Phase 2 — Replace EventStore usage with EventLog

Tasks

- Deprecate direct `EventStore` usage in execution paths.
- Update tool execution and built-in paths to use `&mut dyn EventLog` from `Storage`.
- Keep `EventStore` as an in-memory implementation behind `MemoryStorage`.

Acceptance Criteria

- All existing tests compile with memory storage.
- Event logging behavior unchanged.

---

## Phase 3 — Core: inject Storage

Tasks

- Update `Core` to own `Box<dyn Storage>` (or `Arc<Mutex<dyn Storage>>`).
- Replace `Core::new()` with `Core::new(storage: Box<dyn Storage>)`.
- Update `apps/desktop/src-tauri/src/state.rs` to pass `MemoryStorage::new()`.
- Implement `Core::get_recent_actions(limit)` using event log tail + mapping to `ActionSummary`.

Acceptance Criteria

- Core works end-to-end with memory storage.
- Recent actions return stable summaries.

---

## Phase 4 — Clipboard history via ClipboardStore

Tasks

- Extend execution context to provide access to `ClipboardStore` (via `Storage`).
- Update clipboard tools to capture entries on use (capture-on-use).
- Add bounded history logic in the store (default max 50).

Acceptance Criteria

- `clipboard.list` returns history previews in most-recent-first order.
- History is bounded and de-duplicates consecutive entries.

---

## Phase 5 — Workspace snapshots

Tasks

- Add snapshot save/load hooks in Core:
  - On startup: attempt `storage.snapshots().load()`
  - After execution: `storage.snapshots().save(snapshot)`
- Keep persistence optional in v0 (memory-only ok).

Acceptance Criteria

- Snapshot API works for memory storage.
- Core can restore from snapshot if present.

---

## Phase 6 — ActionSummary shape + redaction

Tasks

- Expand `ActionSummary` to include timestamp, status, duration, tool/app identifiers.
- Map events to summaries using redacted event views.

Acceptance Criteria

- Recent Actions UI can render meaningful items without raw payloads.

---

## Phase 7 — SQLite-ready interface (no implementation yet)

Tasks

- Add a placeholder `SqliteStorage` struct (no implementation in v0).
- Document intended schema (event_log, workspace_snapshot, kv, clipboard_items).

Acceptance Criteria

- Codebase is ready for SQLite without refactoring core logic.

---

## Notes / Decisions

- Storage is injected into Core; no global static storage.
- Event log is append-only; no update/delete.
- Clipboard history lives in storage, not in the platform provider.
- v0 uses in-memory storage; disk persistence is a later phase.
