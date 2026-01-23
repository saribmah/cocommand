---
title: Execution Model
---

Cocommand uses LLMs as planners and orchestrators, not as direct executors.
All system mutations are validated and performed through deterministic tools governed by explicit policies and permissions.

## Pipeline

User Command
→ Capability Routing (retrieve shortlist)
→ LLM Planning (choose capabilities + steps)
→ Authorization (permissions/confirmations)
→ Tool Invocation (system executes)
→ Workspace Patch + Journal Event

## Interaction Modes

### Command Mode (Default)
- Each user input is treated as an independent command.
- No long-term conversational history is retained.

### Follow-Up Mode (Ephemeral)
- Entered after a successful command when new input is likely a continuation.
- Retains references, not full conversation history.

#### Follow-Up Limits
- TTL: 60–120 seconds after last interaction.
- Turns: 1–3 user inputs.
- Scope: entities created or modified by the immediately preceding command.

## Latency Classes

- Instant: purely local, deterministic actions.
- Fast: local compute or cached operations.
- Network-Bound: external API calls.
- Confirmatory: requires user approval.
- Asynchronous: long-running tasks.

## Design Guarantees

- The command bar responds immediately to input.
- Actions that cannot complete instantly provide visible progress feedback.
- No action blocks user input while awaiting network or confirmation.
- Long-running tasks may continue asynchronously after command execution.
