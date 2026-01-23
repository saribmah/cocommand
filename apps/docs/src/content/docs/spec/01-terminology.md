---
title: Terminology
---

Terms are grouped into user-facing and system concepts to avoid ambiguity.

## User-Facing Concepts

### Application
An Application is a logical unit of functionality that Cocommand can interact with to fulfill a user command.

Applications may be:
- System Applications: external apps installed on the user’s system.
- Built-in Applications: apps shipped with Cocommand.
- Custom Applications: apps created via extensions.

From a user’s perspective, all Applications are treated uniformly: they can be referenced, tagged, opened, and used to perform actions.

### Command
A Command is any natural language input entered by the user into the command bar.
Commands express intent, not implementation.

### Tagging
Tagging allows users to explicitly constrain or guide command execution by referencing one or more Applications.
Tagging acts as a routing constraint, not a guarantee of success.

### Virtual Workspace (User Concept)
The Virtual Workspace is a conceptual model that represents the user’s current working context.
It behaves like a virtual window where Applications can be opened, focused, used, and closed.

## System Concepts

### Capability
A Capability represents a specific action or behavior an Application can perform.
Capabilities define what can be done, independent of how the action is executed.

### Capability Router
A system component that maps a user Command to a bounded set of candidate Capabilities/Applications using retrieval and context.
The router runs before the LLM planner to reduce complexity and improve reliability.

### Tool
A Tool is the executable interface through which a Capability is invoked by the LLM.
Tools define input schema, output schema, side effects, and error conditions.

### Kernel Tools
Kernel Tools are privileged Tools that can mutate core system state such as opening/closing Applications or mounting tools.

### Application Tools
Application Tools are Tools exposed by a specific Application and become available only when the Application is active.

### Virtual Workspace (System Definition)
A persistent, structured session state that represents the LLM’s current operating context. The workspace is LLM-readable but not directly writable.

### LLM Kit
The configuration layer that allows users to select and manage the AI models used by Cocommand.
