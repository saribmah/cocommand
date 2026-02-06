# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cocommand (COCO) is an AI-native command bar for macOS. Users type natural-language commands, and the system interprets intent, routes to the correct tool, and returns results or execution plans.

**Core execution pipeline:**
```
User Command → Capability Router → LLM Planner → Permission Layer → Tool Executor → Workspace Patch
```

## Development Commands

### Desktop App (full stack)
```bash
cd apps/desktop
bun install
bun tauri dev
```

### Frontend Only (React/Vite)
```bash
cd apps/desktop
bun dev
```

### Rust Backend
```bash
cd crates/cocommand
cargo check
cargo test
```

### Documentation Site
```bash
cd apps/docs
bun install
bun dev
```

### Build Desktop App
```bash
cd apps/desktop
bun tauri build
```

## Prerequisites

- Bun v1.2.17+
- Rust 2021 edition
- Tauri CLI v2

## Architecture

### Monorepo Structure
- `apps/desktop/` - Tauri + React desktop app
  - `src/` - React frontend (TypeScript)
  - `src-tauri/` - Tauri backend (Rust)
- `crates/cocommand/` - Core Rust library (Axum HTTP server, LLM integration, tool system)
- `crates/platform-macos/` - macOS-specific bindings (objc2)
- `apps/docs/` - Astro documentation site
- `extensions/` - Example extensions

### Backend (Rust - `crates/cocommand`)

**HTTP Server** (`server.rs`) - Axum-based REST API on port 4840:
- `POST /sessions/message` - Process user commands
- `GET /sessions/context` - Session context
- `GET /workspace/applications` - List applications
- `POST /workspace/applications/open` - Open application
- `GET /events` - SSE event stream

**Key modules:**
- `session/` - Session management with UUIDs, application caching
- `workspace/` - Workspace state, config, file-based storage
- `llm/` - LLM service wrapping `llm-kit` (OpenAI-compatible)
- `tool/` - Tool registry (`search_extensions`, `get_extension`, `activate_extension`)
- `application/` - Application registry (system apps, built-ins, extensions)
- `extension/` - Extension host, manifest loading, sandboxed execution
- `bus.rs` - Publish-subscribe event system

### Frontend (React - `apps/desktop/src`)

**State Management** (Zustand stores in `state/`):
- `useCommandBar()` - Command input and results
- `useSessionStore()` - Session context, message history
- `useAiStore()` - LLM response streaming
- `useApplicationStore()` - Available applications
- `useServerStore()` - Backend server info

**Key components:**
- `CommandBar.tsx` - Main command input + results
- `ResultCard.tsx`, `SuggestionList.tsx`, `ConfirmPanel.tsx` - Result rendering

**IPC:** `lib/backend.ts` defines `BASE_URL = http://127.0.0.1:4840`

### Tauri Bridge (`apps/desktop/src-tauri`)

- Global hotkey: `Cmd+O` toggles main window
- Backend server starts with retry logic (3 retries, 200ms delay)
- Commands: `get_workspace_dir_cmd`, `set_workspace_dir_cmd`, `get_server_status_cmd`

## Key Concepts

**Applications:** System apps (macOS), Built-ins (Clipboard, Calculator, Notes), Extensions (user-defined)

**Tools:** Executable interfaces invoked by the LLM with input/output schemas and risk levels (safe/confirm/destructive)

**Workspace:** LLM-readable session state tracking active applications, focused app, and mounted tools. Mutations only via Kernel Tools.

**Extensions:** TypeScript extensions running in sandboxed Deno host, defined by manifest with routing metadata, permissions, and tool declarations.

## Code Style

- **JavaScript/React:** 2-space indentation, `camelCase` variables, `PascalCase` components
- **Rust:** 4-space indentation, idiomatic `snake_case`
- **JSON definitions:** `kebab-case` IDs, semver versions
- **Commits:** Short, imperative, scope-light (e.g., `add planner`, `move planning to backend`)

## Testing

- Desktop E2E tests: `cargo test -p cocommand-desktop` (in `apps/desktop/src-tauri/`)
- Backend unit tests: `cargo test` (in `crates/cocommand/`)
- No dedicated JS test framework yet

## External Dependencies

The project uses a local `llm-kit-*` SDK suite (path: `../../../ai-sdk/llm-kit-*`) for LLM integration. Ensure this sibling directory exists.
