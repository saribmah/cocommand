---
title: Permissions
---

Permissions define what actions are allowed, under what conditions, and with what level of user consent.
They apply at multiple layers and are enforced independently of the LLM.

## Permission Layer

The Permission Layer governs:
- Application integration
- Capability exposure
- Tool execution
- Workspace mutation

## Permission Scope

### Application Permissions
Define what access an Application requires to be installed or used.
Examples: external APIs, file access, network resources, system integrations.

### Capability Permissions
Define consent levels per capability:
- Safe: read-only or non-destructive actions.
- Confirm: modifies user data or triggers side effects.
- Destructive: irreversible or high-impact actions.

### Tool Permissions
Execution-time gate that validates application and capability permissions and enforces context constraints.

## Enforcement Model

- The LLM cannot grant permissions.
- The LLM cannot bypass permission checks.
- All decisions are enforced before tool execution.

If a permission requirement is unmet, the system may request approval, request clarification, or reject execution.

## Design Principles

- Permissions are layered, not monolithic.
- Permissions define maximum authority, not intent.
- The Permission Layer is authoritative; the LLM is advisory.
- Users remain in control of destructive or sensitive actions.
