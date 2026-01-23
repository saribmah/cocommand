---
title: Extensions
---

Cocommand supports user-built Applications via a TypeScript-first extension system executed in a sandboxed Deno host.
Rust remains authoritative for permissions, workspace mutation, and event logging.

## Architecture Overview

- Rust Core (Kernel + Router + Policy)
  - Loads manifests
  - Routes commands
  - Authorizes tool calls
  - Executes tools via Deno host
  - Applies workspace mutations
  - Writes event stream
- Deno Extension Host
  - Loads extension bundles
  - Registers tools/capabilities
  - Runs tool handlers in a constrained runtime

## Manifest (v0)

Extensions declare metadata, routing hints, permissions, and tools.

Key fields:
- id, name, description, entrypoint
- application: app_id, display_name, app_kind, ui
- routing: keywords, examples
- permissions: scope + level
- tools: id, risk_level, input_schema, output_schema

## Execution Contract

- Tool handlers should be pure functions from (args, context) → result when possible.
- Outputs must match declared schemas.
- Tools do not mutate workspace directly.

## Sandboxing

- No filesystem, env, or network access unless explicitly granted by policy.
- No privileged system calls.
- Timeouts and resource limits enforced per tool invocation.

## Lifecycle

1. Install
2. Load
3. Route
4. Authorize
5. Execute
6. Commit
7. Unload/Disable
