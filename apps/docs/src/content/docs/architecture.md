---
title: Architecture
---

## Goals
- Fast, low-friction command execution.
- Clear separation between UI, planning, and execution.
- Extensible integrations for third-party apps and OS services.
- Safe execution with auditability and opt-in automation.

## High-Level Flow
1) Capture: global hotkey opens the command bar.
2) Plan: AI selects an application and a tool (or uses the user-selected app).
3) Execute: the tool runs with validated inputs and permissions.
4) Feedback: results and traces are shown in the UI and stored locally.

## Core Concepts
- Application: a target surface like Spotify, Notetaker, or Finder. Each application exposes tools and capabilities.
- Tool: a single action the app can perform (e.g., `spotify.play`, `notes.create`, `finder.move`).
- Command: a user request expressed in natural language.
- Intent: structured decision about which application + tool to use and with what parameters.
- Workflow: a chain of tool calls (optionally across multiple applications).
- Workspace: virtual container tracking open apps, focused app, and tool availability (see `docs/virtual-workspace.md`).

## Two-Phase Execution Model

The agent executes user commands in two distinct phases:

### Control Phase
- Only `window.*` tools available
- Agent can: list apps, open/close/focus apps, get workspace snapshot
- Purpose: Determine which apps need to be opened before executing actions
- Used when: No apps are currently open in the workspace

### Execution Phase
- `window.*` tools + app-specific tools for open apps
- Agent can: use application tools (e.g., `spotify.play`)
- Triggered when:
  - Apps are already open in the workspace (direct execution)
  - New apps were opened during the Control Phase (transition to execution)

### Phase Selection Logic
1. If workspace is archived → Control Phase only (allows restore)
2. If apps are already open → Direct Execution Phase (enables immediate tool use)
3. If no apps open → Control Phase first, then Execution if apps were opened

This design enables efficient command handling while maintaining intentionality.

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

### Module Responsibilities

| Module | Purpose |
|--------|---------|
| `agent` | Agent loop, prompt assembly, session management |
| `applications` | Application definitions, tool traits, app registry |
| `tool` | Tool set builders, window control tools |
| `workspace` | Virtual workspace state, lifecycle management |
| `server` | HTTP API layer, request handling |
| `storage` | Workspace persistence (file/memory) |
| `llm` | LLM client, configuration, model selection |

### Adding New Applications

To add a new application (e.g., Apple Music):

1. Create `applications/apple_music.rs` as module root
2. Create `applications/apple_music/` directory with:
   - `app.rs`: Implement `Application` trait
   - Tool files: One per tool, implement `Tool` trait
   - `script.rs`: Shared utilities (if needed)
3. Register in `applications/registry.rs`:
   ```rust
   pub fn all_apps() -> Vec<ApplicationDefinition> {
       vec![
           Box::new(spotify::SpotifyApp::default()),
           Box::new(apple_music::AppleMusicApp::default()), // Add here
       ]
   }
   ```
4. Add tool execution routing in `execute_tool()`

## Workspace Integration

The virtual workspace system integrates with the agent through these touchpoints:

### Workspace → Agent Context
```
WorkspaceState → ContextBuilder → AgentContext
     │                │               │
     └── open_apps    │               └── snapshot (LLM-readable)
     └── focused_app  │               └── lifecycle_message
     └── last_active  └── staleness   └── is_archived
```

### Phase → Tool Assembly
```
SessionPhase + WorkspaceState → ToolSet
     │              │               │
     Control ───────│──────────→ window.* only
     Execution ─────┴──────────→ window.* + app tools for open_apps
```

### Lifecycle Flow
```
[Idle] → staleness_level() → [fresh|stale|dormant|archived]
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
               Use as-is    Soft reset LLM    Block + require
                           snapshot data     window.restore
```

## Workspace Lifecycle

The workspace tracks staleness based on idle time:

| Idle Time | Level    | Behavior                              |
|-----------|----------|---------------------------------------|
| < 2h      | fresh    | Use workspace as-is                   |
| 2-24h     | stale    | Refresh ephemeral data, use as-is    |
| 24h-7d    | dormant  | Soft reset (empty LLM snapshot)       |
| > 7d      | archived | Require manual restore                |

## Extensibility
- Add a new application by defining tools + permissions in an integration module.
- Tools become available to the planner via the registry.
- Workflows can stitch tools across applications.

## Safety and Trust
- Default to read-only actions unless confirmed.
- Permission tiers per application and tool.
- Audit log of tool runs and results.
- Archived workspaces block execution until restored.
