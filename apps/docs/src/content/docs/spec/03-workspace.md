---
title: Workspace
---

The Virtual Workspace is a persistent, structured session state that represents the LLM’s current operating context.
It is LLM-readable but not directly writable. All mutations occur via Kernel Tools.

## Responsibilities

- Track open application instances and focus state.
- Track mounted tools for active applications.
- Hold ephemeral context for short-lived follow-ups.
- Provide a consistent state for routing, permissions, and tool execution.

## Workspace Contract (v0)

- At most one focused application instance at a time.
- Every ApplicationInstance has a stable instance_id.
- Only instances with status = active may have application tools mounted.
- Mounted application tools must be associated with exactly one instance_id.
- Kernel tools are always mounted; application tools are mounted only on demand.
- Workspace is updated only via Kernel Tools.
- Permission-denied operations do not mutate workspace state.
- Workspace mutations are atomic: either fully applied or not applied.
- Workspace must be serializable and reloadable without loss of required state.
- Close application is idempotent.
- Open application returns an instance_id and is idempotent only with a stable dedupe key.

## Follow-Up References

Follow-up mode stores references, not full conversation history.
Example fields: last_command, last_result refs (ids), expires_at.

## Persistence Model

The system records an append-only event stream as the source of truth.
A materialized Workspace Snapshot is derived from the stream for fast loading and bounded LLM context.
