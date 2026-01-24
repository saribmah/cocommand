---
title: Architecture
---

## Goals
- Fast, low-friction command execution.
- Clear separation between UI, planning, and execution.
- Extensible integrations for third-party apps and OS services.
- Safe execution with auditability and opt-in automation.

## Module Architecture (opencode-inspired)

The architecture follows opencode's modular pattern: each domain is a self-contained module
with its own submodules, types, and internal organization. Modules expose clean public APIs
through their root files while hiding implementation details.

### Design Principles

1. **Module-per-domain**: Each major concern (agent, tool, workspace, applications) is a module
2. **Flat imports**: Root module re-exports commonly used items for convenience
3. **Internal submodules**: Complex modules split into focused submodules (e.g., `session/phase.rs`)
4. **Trait-based extension**: New apps/tools implement traits, not base classes
5. **Phase-aware tooling**: Tools are assembled based on execution phase

### Directory Structure

```
crates/cocommand/src/
├── lib.rs                  # Public API, re-exports
├── command.rs              # Command module root
├── routing.rs              # Routing module root
├── planner.rs              # Planner module root
├── workspace.rs            # Workspace module root
├── permissions.rs          # Permissions module root
├── tools.rs                # Tools module root
├── events.rs               # Events module root
├── extensions.rs           # Extensions module root
├── builtins.rs             # Builtins module root

├── command/                # User input → intent
│   ├── parser.rs
│   └── tagging.rs

├── routing/                # Capability router
│   ├── index.rs            # keyword / embedding search
│   └── scoring.rs

├── planner/                # LLM planning
│   └── planner.rs

├── workspace/              # Workspace schema + invariants
│   ├── state.rs
│   ├── snapshot.rs
│   └── kernel_tools.rs

├── permissions/            # Permission layer
│   ├── scopes.rs
│   ├── risk.rs
│   └── enforcement.rs

├── tools/                  # Tool abstractions
│   ├── registry.rs
│   └── invocation.rs

├── events/                 # Event stream & observability
│   ├── event.rs
│   ├── journal.rs
│   └── replay.rs

├── extensions/             # Deno extension host interface
│   ├── manifest.rs
│   ├── rpc.rs
│   └── lifecycle.rs

├── builtins/               # Built-in applications
│   ├── notes.rs
│   ├── clipboard.rs
│   └── composer.rs

└── latency.rs              # Latency classes + execution modes

apps/desktop/src-tauri/src/
├── lib.rs                 # Tauri bootstrap + plugin setup
├── main.rs                # Entry point
└── window.rs              # Window commands and behavior

crates/platform-macos/src/
└── lib.rs                 # macOS integrations (NSWorkspace/EventKit)
```
