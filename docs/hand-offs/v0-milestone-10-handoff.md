---
title: v0 Milestone 10 Handoff
status: pending
---

# AI Implementation Handoff — Core-10

## Context

You are implementing **Cocommand**, a command-first desktop application with an AI-driven core.
All architecture, terminology, and behavior must follow the canonical documentation.

You are currently implementing:

**`Core-10 — Extension host v0 (Deno) + RPC + sample extension`**

This milestone is part of v0 and must not include v1+ features.

---

## Scope (STRICT)

### You MAY:

- Create or modify files listed under **Allowed Files**.
- Implement only the tasks explicitly listed in **Task List**.
- Add tests only for the behavior described in **Acceptance Criteria**.
- Use existing naming conventions and module layout rules.

### You MUST NOT:

- Modify files outside **Allowed Files**.
- Introduce additional services beyond the Deno host specified here.
- Add UI logic or Tauri dependencies.
- Implement future milestones (desktop UI integration, macOS integrations). 

If anything is unclear, stop and ask for clarification.

---

## References (READ FIRST)

- `apps/docs/src/content/docs/spec/06-extensions.md`
- `apps/docs/src/content/docs/spec/05-routing.md`
- `apps/docs/src/content/docs/tasks/v0.md`
- `apps/docs/src/content/docs/tasks/v0-core.md`

---

## Task List

Implement only the following tasks:

- [ ] Implement Deno extension host process:
  - load manifest
  - register tools
  - invoke tool handler
- [ ] Implement RPC protocol (stdio JSON-RPC recommended for v0).
- [ ] Implement Rust-side extension manager:
  - install/load/unload
  - tool catalog sync
  - routing metadata ingestion
- [ ] Create one sample extension:
  - `my_app.create_ticket` returning safe output

---

## Allowed Files / Folders

You may create or modify only the following:

```text
crates/cocommand/src/extensions.rs
crates/cocommand/src/extensions/manifest.rs
crates/cocommand/src/extensions/rpc.rs
crates/cocommand/src/extensions/lifecycle.rs
apps/extension-host/
apps/extension-host/main.ts
apps/extension-host/protocol.ts
apps/extension-host/loader.ts
extensions/sample-my-app/
```

If a required change would affect files outside this list, stop and report.

---

## File Structure Constraints

- Use **file-based modules** (`module.rs` + `module/` directories).
- Do **not** use `mod.rs`.
- Public types must have brief doc comments.
- Keep dependencies minimal and platform-agnostic (no Tauri imports).

---

## Behavioral Requirements

- Rust core can invoke an extension tool and receive JSON result.
- Extension tools appear in routing candidates.
- Host communication is via stdio JSON-RPC.

---

## Acceptance Criteria

- [ ] Extension tool invocation succeeds end-to-end.
- [ ] Routing sees extension metadata.
- [ ] `cargo check --manifest-path crates/cocommand/Cargo.toml` passes.
- [ ] `cargo test --manifest-path crates/cocommand/Cargo.toml` passes.

---

## Definition of Done

- All tasks implemented.
- All tests in the checklist pass.
- No unrelated refactors or cleanups.
- No TODOs remain for this milestone.

---

## Test Checklist

- [ ] `cargo check --manifest-path crates/cocommand/Cargo.toml`
- [ ] `cargo test --manifest-path crates/cocommand/Cargo.toml`
- [ ] Integration test: spawn Deno host and invoke tool
- [ ] Timeout test: host hangs → invocation times out

---

## Output Expectations

When finished, respond with:

1. Summary of changes
2. Files modified/created
3. Tests added or updated
4. Known limitations (if any)

---

## Failure Handling

If you encounter missing spec details or ambiguity, stop immediately and ask a clarification question.
