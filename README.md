# cocommand.ai

AI-native command bar for macOS. A single entry point for natural-language commands — type what you want, and Cocommand interprets intent, routes to the correct tool, and returns a result or execution plan.

## Prerequisites

- [Bun](https://bun.sh/) v1.2.17+
- [Rust](https://rustup.rs/) (2021 edition)
- [Tauri CLI v2](https://v2.tauri.app/)

## Getting Started

```bash
# Install dependencies
bun install

# Run the desktop app in development
bun --cwd apps/desktop tauri dev

# Production build
bun --cwd apps/desktop tauri build
```

## Architecture

```
React UI  →  Tauri IPC  →  Core Engine (Rust)  →  Platform (macOS)
```

- **Frontend** (`apps/desktop/src/`) — React 19 + Vite. CommandBar input, result cards, confirmation panels.
- **Tauri layer** (`apps/desktop/src-tauri/`) — IPC commands, window management (transparent, always-on-top, 720x180), global shortcut.
- **Core engine** (`crates/cocommand/`) — The brain. Parses commands, routes intent, plans tool calls, enforces permissions, manages session workspace state.
- **Platform** (`crates/platform-macos/`) — macOS-specific integrations.

### Command Pipeline

User text → parse → route (intent matching) → plan (tool calls) → execute → CoreResponse

### Built-in Applications

Calculator, Clipboard, Notes — with an extension system for adding more.

## Project Structure

```
apps/
  desktop/          Tauri + React desktop app
  docs/             Documentation site
crates/
  cocommand/        Core Rust engine
  platform-macos/   macOS platform layer
extensions/         Extension template
docs/hand-offs/     Milestone handoff documents
```

## Documentation

- `Cocommand-Technical-Documentation.md` — Full product spec, terminology, and system concepts
- `docs/hand-offs/` — Per-milestone implementation details
- `CLAUDE.md` — Development commands and architecture reference
