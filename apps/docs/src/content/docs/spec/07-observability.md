---
title: Observability
---

Cocommand includes structured observability to support trust, debugging, and reproducible execution.

## Event Stream

The system records an append-only event stream including:
- user messages
- tool call requests
- tool results
- permission decisions
- workspace patches
- errors

## Tool Invocation Record

Each tool invocation records:
- tool identifier and caller context
- timestamps and duration
- authorization outcome
- redacted inputs/outputs plus hashes
- error codes and sanitized messages
- workspace hashes before/after
- runtime provenance (model/provider, prompt version, router version)

## Redaction

Sensitive user content is redacted by default.
Development mode may optionally relax redaction for local debugging.

## User-Facing Surfaces (v0)

- Recent Actions history
- Undo when supported
- Extension developer inspector
