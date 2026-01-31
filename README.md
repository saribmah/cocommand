# <img src="apps/desktop/public/logo_dark.png" width="45" height="45" />

![cocommand](README-header.png)

COCO is an AI-native command bar for macOS. A single entry point for natural-language commands — type what you want, and Cocommand interprets intent, routes to the correct tool, and returns a result or execution plan.

## Prerequisites (development)

- [Bun](https://bun.sh/) v1.2.17+
- [Rust](https://rustup.rs/) (2021 edition)
- [Tauri CLI v2](https://v2.tauri.app/)

## Getting Started

### Desktop app (dev)

```sh
cd apps/desktop
bun install
bun tauri dev
```

### Frontend only (dev)

```sh
cd apps/desktop
bun dev
```

### Backend checks (dev)

```sh
cd crates/cocommand
cargo check
cargo test
```

### Docs (dev)

```sh
cd apps/docs
bun install
bun dev
```

## Documentation

- Docs site: `apps/docs/`
- `CLAUDE.md` — Development commands and architecture reference
- `apps/docs/src/content/docs/quick-start.mdx` — Installation overview
- `apps/docs/src/content/docs/codebase/` — Module-by-module codebase docs
