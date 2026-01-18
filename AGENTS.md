# Repository Guidelines

## Project Structure & Module Organization
- `apps/desktop-tauri/`: Tauri + React app (UI, IPC, window behavior). Frontend code in `apps/desktop-tauri/src/` and Rust backend in `apps/desktop-tauri/src-tauri/`.
- `packages/`: Shared domain logic and schemas.
  - `packages/core/`: Types, registries, planner interfaces.
  - `packages/commands/` and `packages/workflows/`: JSON schemas, validators, examples.
  - `packages/llm/`: Heuristic planner implementation.
- `docs/`, `SPEC.md`, `ARCHITECTURE.md`: Product concept and architecture context.

## Build, Test, and Development Commands
- `bun tauri dev` (run in `apps/desktop-tauri/`): Start the desktop app with Vite + Rust backend.
- `bun dev` (run in `apps/desktop-tauri/`): Start the Vite frontend only.
- `bun tauri build` (run in `apps/desktop-tauri/`): Build the desktop app.
- `bun run validate:command` (repo root): Validate command example JSON against schema.
- `bun run validate:workflow` (repo root): Validate workflow example JSON against schema.

## Coding Style & Naming Conventions
- JavaScript/React: 2-space indentation; prefer camelCase for variables/functions; components in PascalCase (e.g., `App.jsx`).
- Rust: 4-space indentation; follow idiomatic `snake_case` for functions and variables.
- JSON definitions: use `kebab-case` IDs (e.g., `daily-wrap`) and keep `version` as semver strings.

## Testing Guidelines
- No dedicated test framework is set up yet.
- Use the validation scripts (`validate:command`, `validate:workflow`) to catch schema issues.

## Commit & Pull Request Guidelines
- Commit messages are short, imperative, and scope-light (e.g., `add planner`, `move planning to backend`).
- PRs should include a brief summary, testing performed, and relevant screenshots for UI changes.

## Architecture Notes
- Planning happens in the backend via Tauri commands; UI calls `plan_command` and (for workflows) `run_workflow`.
- Command/workflow JSON is loaded from bundled examples and user data dirs under the app data path.
