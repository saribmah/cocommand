# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Cocommand

AI-native command bar for macOS (like Spotlight/Raycast with LLM-driven orchestration). Users type natural-language commands → the system interprets intent → routes to the correct tool → returns an execution plan or result.

## Build & Development Commands

**Prerequisites:** Bun (v1.2.17), Rust (2021 edition), Tauri CLI v2

```bash
# Install dependencies
bun install

# Frontend dev server (Vite)
bun --cwd apps/desktop dev

# Full desktop app (Tauri + Vite)
bun --cwd apps/desktop tauri dev

# Production build
bun --cwd apps/desktop tauri build

# Docs site
bun docs:dev
bun docs:build

# Rust tests (core crate)
cargo test -p cocommand

# Single test
cargo test -p cocommand <test_name>

# Rust build check
cargo check -p cocommand
cargo check -p cocommand-desktop
```

## Architecture

```
┌─────────────────────────────────────────┐
│  React UI (apps/desktop/src/)           │
│  CommandBar → ResultCard/ConfirmPanel   │
├─────────────────────────────────────────┤
│  Tauri IPC (apps/desktop/src-tauri/)    │
│  commands.rs ↔ state.rs ↔ window.rs    │
├─────────────────────────────────────────┤
│  Core Engine (crates/cocommand/src/)    │
│  core.rs → routing → planner → tools   │
├─────────────────────────────────────────┤
│  Platform (crates/platform-macos/)      │
└─────────────────────────────────────────┘
```

### Core Engine (crates/cocommand/src/)

The `Core` struct in `core.rs` is the primary facade. All command processing flows through `Core::submit_command()`.

**Pipeline:** User text → `command::parse()` → `Router::route()` → `Planner` → Tool execution → `CoreResponse`

Key modules:
- **command/** — Parsing and normalization of natural-language input
- **routing/** — Intent matching to find the right application/tool
- **planner/** — Generates tool call plans from routed candidates
- **workspace/** — Session-scoped state (follow-up mode, confirmation pending, app instances)
- **tools/** — Registry, schema definitions, executor, invocation tracking
- **permissions/** — Per-tool permission enforcement
- **builtins/** — Built-in apps (Calculator, Clipboard, Notes)
- **extensions/** — Extension loading system
- **events/** — Event bus

### CoreResponse Types

All responses cross the Tauri boundary as one of:
- `Artifact` — Renderable result with optional actions
- `Preview` — Read-only display (e.g., last note)
- `Confirmation` — Asks user to confirm a risky action
- `Error` — User-displayable error message

### Workspace Model

Sessions use a virtual workspace with:
- Follow-up mode (90s TTL, max 3 turns) for contextual multi-turn interaction
- Confirmation pending state for risky operations
- Application instance mounting with scoped tools

### Frontend (apps/desktop/src/)

- **components/** — CommandBar, ResultCard, ConfirmPanel, MarkdownView, SuggestionList
- **state/** — State management (commandbar.ts)
- **lib/** — IPC bridge to Tauri backend
- **types/** — Shared TypeScript types

### Tauri Layer (apps/desktop/src-tauri/src/)

- **commands.rs** — `#[tauri::command]` handlers exposed to frontend
- **state.rs** — Holds the `Core` instance in Tauri managed state
- **window.rs** — Window management (720x180, transparent, always-on-top)

## Monorepo Structure

Uses Bun workspaces. Key paths:
- `apps/desktop` — Main Tauri + React desktop app
- `apps/docs` — Documentation site
- `crates/cocommand` — Core Rust engine (the brain)
- `crates/platform-macos` — macOS platform integrations
- `extensions/` — Extension template

## Key Dependencies

**Rust:** axum 0.7, tokio 1 (multi-thread), serde/serde_json, uuid v4
**Frontend:** react 19, vite 7, @tauri-apps/api 2
**Desktop:** tauri 2 (macos-private-api), tauri-plugin-global-shortcut 2

## Conventions

- The core HTTP server runs on port 4840 (health endpoint)
- Tauri window uses `macos-private-api` for transparent/borderless styling
- Global shortcut for toggling the command bar
- All cross-boundary types live in `crates/cocommand/src/types.rs`
- Documentation milestones in `docs/hand-offs/`
