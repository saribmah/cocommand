# Repository Guidelines

## Project Structure & Module Organization
- `apps/desktop/`: Tauri + React desktop app. Frontend code lives in `apps/desktop/src/` and the Rust backend in `apps/desktop/src-tauri/`.
- `crates/`, `internal/`, `extensions/`: Shared Rust crates and internal tooling.
- `docs/`, `SPEC.md`, `ARCHITECTURE.md`, `Cocommand-Technical-Documentation.md`: Product and architecture references.
- Example command/workflow JSON is bundled and also stored in user data directories at runtime (app data path).

## Build, Test, and Development Commands
Run commands from the repo root unless noted.
- `bun tauri dev` (run in `apps/desktop/`): Start the full desktop app (Vite + Rust backend).
- `bun dev` (run in `apps/desktop/`): Run the Vite frontend only.
- `bun tauri build` (run in `apps/desktop/`): Build the desktop app.
- `bun run validate:command`: Validate command example JSON against schema.
- `bun run validate:workflow`: Validate workflow example JSON against schema.

## Coding Style & Naming Conventions
- JavaScript/React: 2-space indentation, `camelCase` for variables/functions, `PascalCase` for components (e.g., `App.jsx`).
- Rust: 4-space indentation, idiomatic `snake_case` for functions and variables.
- JSON definitions: use `kebab-case` IDs (e.g., `daily-wrap`) and keep `version` as semver strings.
- Prefer small, focused UI components and explicit IPC boundaries between the React UI and Tauri backend.

## Testing Guidelines
- No dedicated test framework is set up yet.
- Use the validation scripts (`validate:command`, `validate:workflow`) to catch schema regressions.
- If you add tests in a new area, document how to run them in this file and in the PR description.

## Commit & Pull Request Guidelines
- Commit messages are short, imperative, and scope-light (e.g., `add planner`, `move planning to backend`).
- PRs should include: a brief summary, testing performed, and screenshots for UI changes.
- Note any schema updates or example JSON changes explicitly.

## Architecture Overview
- Planning happens in the backend via Tauri commands. The UI calls `plan_command` and, for workflows, `run_workflow`.
- Command/workflow JSON is loaded from bundled examples and user data directories under the app data path.

## Agent-Specific Notes
- Follow repository guidelines in this document and `CLAUDE.md` when updating files.
- Keep changes minimal and aligned with existing patterns unless otherwise requested.
