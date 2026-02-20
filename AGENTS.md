# AGENTS.md

Agent handbook for working in the `cocommand` monorepo.
Use this as the default execution and style guide for code agents.

## Repository Map
- `apps/desktop/`: Tauri + React desktop app.
- `apps/desktop/src/`: React frontend (feature-oriented folders).
- `apps/desktop/src-tauri/`: Tauri Rust bridge and app lifecycle.
- `crates/cocommand/`: Core Rust backend (Axum server, workspace, tools, sessions).
- `crates/platform-macos/`: macOS-specific bindings.
- `apps/docs/`: Astro docs site.
- `packages/ui/`: shared UI package.
- `packages/demo/`: demo package.
- `extensions/`: example extension assets.

## Prerequisites
- Bun `1.2.17+`
- Rust 2021 edition
- Tauri CLI v2
- Local sibling SDKs required by Rust core: `../../../ai-sdk/llm-kit-core`, `../../../ai-sdk/llm-kit-openai-compatible`, `../../../ai-sdk/llm-kit-provider`, `../../../ai-sdk/llm-kit-provider-utils`

## Build, Run, and Check Commands
Run from repo root unless noted.

### Desktop App (React + Tauri)
- Full dev app: `bun --cwd apps/desktop run tauri dev`
- Frontend only: `bun --cwd apps/desktop run dev`
- Production bundle: `bun --cwd apps/desktop run tauri build`
- Frontend production build: `bun --cwd apps/desktop run build`
- Preview frontend build: `bun --cwd apps/desktop run preview`

### Rust Core (`crates/cocommand`)
- Type/check compile: `cargo check --manifest-path crates/cocommand/Cargo.toml`
- Run all tests: `cargo test --manifest-path crates/cocommand/Cargo.toml`

### Tauri Bridge (`apps/desktop/src-tauri`)
- Build/check crate: `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
- Run tests in crate: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`
- Note: this crate currently has no committed `#[test]` functions.

### Docs Site (`apps/docs`)
- Dev server: `bun --cwd apps/docs run dev`
- Build docs: `bun --cwd apps/docs run build`
- Preview docs: `bun --cwd apps/docs run preview`

### Root Scripts
- `bun run docs:dev`
- `bun run docs:build`
- `bun run docs:preview`

## Linting, Formatting, and Static Checks
- There is no repo-wide ESLint/Prettier/Biome config committed right now.
- TypeScript strictness is enabled in app configs (`strict`, `noUnusedLocals`, `noUnusedParameters`).
- Rust formatting/check conventions:
  - `cargo fmt --manifest-path crates/cocommand/Cargo.toml`
  - `cargo clippy --manifest-path crates/cocommand/Cargo.toml -- -D warnings` (if requested)
- Prefer matching existing formatting in touched files when no formatter is configured.

## Single-Test Workflows (Important)
Primary test surface is Rust.

### Run one test by name substring
- Core crate:
  - `cargo test --manifest-path crates/cocommand/Cargo.toml <test_name_substring>`

### Run one exact test
- Core crate:
  - `cargo test --manifest-path crates/cocommand/Cargo.toml server::tests::start_binds_random_port -- --exact --nocapture`

### Run one module test group
- Core crate:
  - `cargo test --manifest-path crates/cocommand/Cargo.toml server::tests -- --nocapture`

### Run tests for one file-backed module (example)
- Filesystem built-in tests:
  - `cargo test --manifest-path crates/cocommand/Cargo.toml extension::builtin::filesystem::tests -- --nocapture`

### Tauri crate single-test pattern
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml <test_name_substring> -- --exact --nocapture`

## Architecture Notes for Agents
- Local HTTP server starts inside Tauri app state; server binds random local port.
- Frontend obtains server metadata through Tauri invoke wrappers in `apps/desktop/src/lib/ipc.ts`.
- Session command path is SSE-driven (`/sessions/command`) and streams partial events.
- Workspace and extension data are persisted via Rust workspace/storage modules.
- Keep payloads serializable and transport-friendly across IPC and HTTP boundaries.

## Code Style and Conventions

### Cross-Language
- Keep functions cohesive and side effects explicit.
- Prefer small helpers for parsing/normalization logic.
- Do not add new dependencies unless necessary and justified.
- Avoid broad refactors unless explicitly requested.

### TypeScript / React
- Indentation: 2 spaces.
- Strings: use double quotes.
- Semicolons: keep semicolons (current codebase style).
- Naming:
  - variables/functions: `camelCase`
  - React components/types/interfaces: `PascalCase`
  - many file names are feature-oriented lowercase with dots (for example `command.view.tsx`, `session.store.ts`).
- Imports:
  - external packages first, then internal modules, then local styles.
  - use `import type` for type-only imports.
  - keep side-effect imports explicit (for example CSS/UI package imports).
- Types:
  - avoid `any`; prefer explicit interfaces/unions and `unknown` with narrowing.
  - keep public boundary types stable and explicit.
  - respect strict TS settings; do not suppress errors casually.
- State/data flow:
  - use existing Zustand/context provider patterns under `apps/desktop/src/features/*`.
  - avoid duplicating server state in multiple stores unless intentional.
- Error handling:
  - normalize unknown errors before displaying.
  - surface user-facing, actionable messages in UI flows.
  - handle async failures (`try/catch`, rejected promises) close to boundary calls.

### Rust
- Indentation: 4 spaces.
- Naming: idiomatic `snake_case` for functions/modules, `PascalCase` for types.
- Error handling:
  - prefer `Result<T, E>` returns and propagate with `?`.
  - in core crate, use `CoreResult<T>`/`CoreError` where appropriate.
  - at Tauri command boundary, return `Result<_, String>` and map errors clearly.
  - avoid `unwrap`/`expect` in runtime paths (tests are fine).
- Imports:
  - group by std / external crates / local crate modules.
  - keep import lists tidy and deterministic.
- Concurrency/runtime:
  - use `tokio` primitives already in use (`oneshot`, `watch`, async tasks).
  - ensure shutdown paths are explicit and idempotent.

### JSON and Config Shapes
- IDs: `kebab-case`.
- Versions: semver strings.
- Keep schemas backward-compatible unless migration is part of the task.

## Testing Expectations
- Add/adjust tests for meaningful behavior changes in Rust modules.
- Prefer targeted test runs during iteration, then broader suite before handoff.
- If no automated test exists for a changed area, mention manual verification steps.

## Cursor and Copilot Rules
- `.cursor/rules/`: not present.
- `.cursorrules`: not present.
- `.github/copilot-instructions.md`: not present.
- If these files are added later, treat them as higher-priority agent instructions.

### VERY IMPORTANT TO REMEMBER
We're using module/ and module.rs structure. No mod.rs. NEVER CREATE mod.rs file
