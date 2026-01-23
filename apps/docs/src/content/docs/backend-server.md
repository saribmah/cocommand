---
title: Backend Server Design
---

## Goal
Provide a standalone backend service that owns planning, execution, and data storage, so any UI (Tauri, iOS, web) can connect over a stable API.

## Responsibilities
- **Plan**: call LLM provider, return a structured plan `{appId, toolId, inputs}`.
- **Execute**: run tools/workflows on the host machine.
- **Registry**: expose available applications and tools.
- **Storage**: persist workflows, history, settings.
- **Safety**: enforce permissions and maintain audit logs.

## Process Layout
```
backend/
  src/
    api/             # HTTP/WS or gRPC handlers
    planner/         # LLM client + prompt + parsing
    registry/        # applications + tools
    executor/        # tool/workflow runner
    storage/         # local persistence
    auth/            # API tokens and session control
```

## API Surface (HTTP + WebSocket)
### REST Endpoints
- `POST /plan`
  - input: `{ text, context?, appId? }`
  - output: `{ intent, plan, confidence }`
- `POST /execute`
  - input: `{ planId }` or `{ toolId, inputs }`
  - output: `{ status, outputs, errors? }`
- `GET /tools`
  - output: list of apps and tools with schemas.
- `GET /workflows`
  - output: saved workflows.
- `POST /workflows`
  - create/update workflow.
- `DELETE /workflows/{id}`
- `GET /history`
  - recent executions.

### WebSocket Events (optional)
- `execution.started`
- `execution.progress`
- `execution.completed`
- `execution.failed`

## Client Connection Model
- **Desktop UI**: connects to `http://localhost:PORT`.
- **iOS/Web**: connects over LAN with token auth or through a secure relay later.
- All clients share the same API contract; no direct access to OS APIs.

## Data Model (minimal)
- **Application**: `{ id, name, tools[] }`
- **Tool**: `{ id, name, description, inputsSchema, permissions }`
- **Plan**: `{ id, intent, steps[] }`
- **Workflow**: `{ id, name, steps[] }`
- **ExecutionLog**: `{ id, planId, status, outputs, createdAt }`

## Security & Auth
- Local-only by default.
- Token-based auth for remote clients.
- Permission checks per tool and workflow.
- Audit log of every execution.

## Implementation Steps
1) **Define API contract** (OpenAPI/JSON schema).
2) **Build server** (Rust + Axum or Node + Fastify).
3) **Implement registry** (static tool list first).
4) **Add planner** (LLM call + structured output).
5) **Add executor** (tool dispatch).
6) **Wire UI** to `/plan` and `/execute`.

## Example Flow
1) UI sends `POST /plan` with text: "Play my focus playlist".
2) Planner returns `{ appId: "spotify", toolId: "spotify.playPlaylist", inputs: { uri: "..." } }`.
3) UI calls `POST /execute` to run the plan.
4) Backend executes and returns output.
