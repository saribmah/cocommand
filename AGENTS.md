# Repository Guidelines for Agents

This file is the primary, concise handbook for automated coding agents working in this repo.
Keep changes minimal, follow established patterns, and update this file when new tooling is added.

## Project Map
- `apps/desktop/`: Tauri + React desktop app.
  - `apps/desktop/src/`: React frontend (TypeScript).
  - `apps/desktop/src-tauri/`: Tauri backend (Rust).
- `crates/cocommand/`: Core Rust library (server, tools, workspace).
- `crates/platform-macos/`: macOS bindings.
- `apps/docs/`: Astro documentation site.
- `extensions/`: Example extensions.
- `docs/`, `SPEC.md`, `ARCHITECTURE.md`, `internal/Cocommand-Technical-Documentation.md`: references.
- Example command/workflow JSON is bundled and stored in app data directories at runtime.
- Frontend state lives in `apps/desktop/src/state/` (Zustand stores).
- Shared TS types live in `apps/desktop/src/types/`.

## Prerequisites
- Bun v1.2.17+
- Rust 2021 edition
- Tauri CLI v2
- Local `ai-sdk/llm-kit-*` sibling directory exists (see `crates/cocommand/Cargo.toml`).

## Build, Dev, and Test Commands
Run commands from repo root unless noted.

### Desktop App (Vite + Rust via Tauri)
- Dev (full stack):
  - `cd apps/desktop`
  - `bun install`
  - `bun tauri dev`
- Frontend-only dev:
  - `cd apps/desktop`
  - `bun dev`
- Build desktop app:
  - `cd apps/desktop`
  - `bun tauri build`

### Backend (Rust crate)
- `cd crates/cocommand`
- `cargo check`
- `cargo test`

### Tauri Bridge Tests
- `cd apps/desktop/src-tauri`
- `cargo test -p cocommand-desktop`

### Docs Site (Astro)
- `cd apps/docs`
- `bun install`
- `bun dev`
- `bun build`

### Root Scripts (Docs wrappers)
- `bun run docs:dev`
- `bun run docs:build`
- `bun run docs:preview`

### Single Test Examples
- Rust unit/integration (crate):
  - `cd crates/cocommand`
  - `cargo test <test_name_substring>`
- Rust unit/integration with output:
  - `cd crates/cocommand`
  - `cargo test <test_name_substring> -- --nocapture`
- Tauri bridge (desktop):
  - `cd apps/desktop/src-tauri`
  - `cargo test -p cocommand-desktop <test_name_substring>`

### Linting/Formatting
- No repo-level JS/TS lint or format scripts are configured.
- Rust formatting uses standard `rustfmt` defaults if you run it (`cargo fmt`).
- Keep formatting consistent with existing files; do not introduce new formatters unless requested.

## Architecture Notes
- Planning happens in the backend via Tauri commands.
- The UI calls `plan_command` and, for workflows, `run_workflow`.
- IPC boundaries are explicit: keep Tauri command types simple and serializable.
- Desktop UI talks to the backend server at `http://127.0.0.1:4840`.
- Command/workflow JSON is loaded from bundled examples and user data directories under the app data path.

## Code Style Guidelines

### General
- Make focused changes; avoid sweeping refactors unless requested.
- Prefer small, single-purpose components and functions.
- Keep changes aligned with existing folder/module structure.

### TypeScript/React
- Indentation: 2 spaces; use semicolons and double quotes.
- Naming: `camelCase` for variables/functions, `PascalCase` for components.
- Types: strict TS is enabled; avoid `any` and prefer explicit types for exported APIs.
- Imports:
  - External deps first, then local modules; keep side-effect CSS imports near the top.
  - Use `type` imports for type-only symbols (e.g., `import type { Foo } from "..."`).
- Components:
  - Use function components; keep helpers above the component definition.
  - Keep state in Zustand stores under `apps/desktop/src/state/`.
- File naming:
  - Components use `PascalCase.tsx` filenames.
  - Store modules use `camelCase` filenames under `apps/desktop/src/state/`.
  - Type definitions live in `apps/desktop/src/types/`.
- Errors:
  - Handle async failures with `.catch`/`try` and convert to user-facing error results.
  - Avoid throwing across IPC boundaries; surface friendly messages instead.
- IPC:
  - Keep payloads JSON-serializable and small.
  - Convert transport errors to friendly UI results.

### Rust
- Indentation: 4 spaces; use idiomatic `snake_case`.
- Error handling:
  - Prefer `CoreResult<T>`/`CoreError` in the core crate.
  - At Tauri command boundaries, map errors to `Result<_, String>`.
  - Avoid `unwrap`/`expect` in runtime paths; use `?` with context instead.
- Imports: standard `use` groups (std, external crates, local crate) with blank lines between groups.
- Modules:
  - Keep new functionality in focused modules (e.g., `tool/`, `workspace/`, `session/`).
  - Favor explicit types at public boundaries.

### JSON Definitions
- IDs use `kebab-case` (e.g., `daily-wrap`).
- `version` fields use semver strings.

## Testing Guidelines
- No dedicated JS test framework exists yet.
- Add new tests only when needed and document how to run them here.
- Desktop E2E (Tauri IPC/core flow) tests live under `apps/desktop/src-tauri/`.
- Backend unit/integration tests live under `crates/cocommand/`.

## Repo Conventions
- Commit messages: short, imperative, scope-light (e.g., `add planner`).
- PRs should include: summary, testing performed, and screenshots for UI changes.
- Note schema updates or example JSON changes explicitly.
- Keep example JSON changes minimal and traceable.

## Documentation References
- `CLAUDE.md`: development commands and architecture reference.
- `apps/docs/src/content/docs/`: module-by-module docs and quick start.
- `SPEC.md` and `ARCHITECTURE.md`: design and architectural constraints.

## Agent-Specific Rules
- Follow this file and `CLAUDE.md`.
- Do not revert unrelated local changes.
- Keep IPC boundaries explicit between React UI and Tauri backend.

## Cursor/Copilot Rules
- No `.cursor/rules/`, `.cursorrules`, or `.github/copilot-instructions.md` found in this repo.
