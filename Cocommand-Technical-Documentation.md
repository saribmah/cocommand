# Cocommand Technical Documentation

Author: [Sarib Mahmood](mailto:saribmahmood55@gmail.com)  
Date: Jan 21, 2026

[Idea](#idea)

[Terminology](#terminology)

[User-Facing Concepts](#user-facing-concepts)

[Application](#application)

[Command](#command)

[Tagging](#tagging)

[Virtual Workspace (User Concept)](#virtual-workspace-\(user-concept\))

[System Concepts](#system-concepts)

[Capability](#capability)

[Tool](#tool)

[Kernel Tools](#kernel-tools)

[Application Tools](#application-tools)

[Virtual Workspace (System Definition)](#virtual-workspace-\(system-definition\))

[LLM Kit](#llm-kit)

[Permissions & Authorization](#permissions-&-authorization)

[Permission Layer (System Concept)](#permission-layer-\(system-concept\))

[Permission Scope](#permission-scope)

[Application Permissions](#application-permissions)

[Capability Permissions](#capability-permissions)

[Tool Permissions](#tool-permissions)

[Permission Enforcement Model](#permission-enforcement-model)

[Permission Declarations](#permission-declarations)

[Design Principles](#design-principles)

[Mental Model](#mental-model)

[Open Questions](#open-questions)

# 

# Idea {#idea}

An AI-first command bar (launch bar) application, LLM-driven orchestration within a permissioned tool runtime, to significantly accelerate common user tasks. The primary goal is to help users execute simple, multi-step actions in moments, reducing the time spent on these tasks from approximately 10-15 seconds to perceived-instant.

# System Overview

Cocommand executes commands through a fixed pipeline that keeps the LLM advisory and the system authoritative:

Command Input → Capability Router (shortlist) → LLM Planner (select + plan) → Permission Layer (authorize/confirm) → Tool Executor (run tools) → Workspace Patch + Journal Event.

This pipeline ensures routing is bounded, permissions are enforced before execution, and every state change is recorded.

# Terminology {#terminology}

This section defines the core concepts used throughout Cocommand.  
Terms are grouped into **User-Facing Concepts** and **System Concepts** to avoid ambiguity between product language and internal architecture.

---

## User-Facing Concepts {#user-facing-concepts}

These are concepts users directly interact with or reason about.

### Application {#application}

An **Application** is a logical unit of functionality that Cocommand can interact with to fulfill a user command.

Applications may be:

* **System Applications**  
  External applications installed on the user’s system (e.g. Spotify, Browser, Terminal).  
* **Built-in Applications**  
  Applications shipped with Cocommand (e.g. Clipboard, Calculator, Notes).  
* **Custom Applications**  
  User-defined applications created via extensions. These typically expose specific actions or workflows to Cocommand.

From a user’s perspective, all Applications are treated uniformly: they can be referenced, tagged, opened, and used to perform actions.

#### AI-Native Built-ins

In addition to standard utilities, Cocommand includes at least one AI-native built-in designed to demonstrate multi-step reasoning, context synthesis, and follow-up workflows.

##### Option 1: Context Composer

A built-in that turns scattered context into something useful.  
Example commands:

* “Turn my last 20 clipboard items into meeting notes”  
* “Summarize what I’ve copied today into a task list”  
* “Create a follow-up email from what I just wrote”

##### Option 2: Intent Transformer

A built-in that converts informal input into structured artifacts.  
Examples:

* “Turn these bullets into a Jira ticket”  
* “Create a PR description from this diff”  
* “Rewrite this note as a spec”

##### Option 3: Daily Digest

A built-in that synthesizes recent activity.  
Examples:

* “Summarize what I worked on today”  
* “What did I copy most today?”  
* “Create a standup update from today’s notes”

##### How to introduce this without blowing scope

You don’t need a new “app” UI.  
Make it:

* a built-in Application  
* with 3–5 high-quality Capabilities  
* heavily documented as examples

##### What kind of Application is this?

This is the key nuance.

**Context Composer is a *headless, AI-native Application*.**

That means:

* no persistent UI panel  
* no window users “open”  
* it exists primarily as a *capability provider*  
* it composes other apps’ outputs

This is an important category you should embrace.

You now have (implicitly):

* UI-centric applications (Notes, Clipboard)  
* External system apps (Spotify)  
* **Orchestrator applications** (Context Composer)

##### What Context Composer actually *does* (in your model)

It does **not** own data.

It:

* reads workspace context  
* reads outputs from other applications  
* synthesizes new artifacts  
* writes results via other apps (Notes, Mail, etc.)

So its capabilities look like:

* `compose.summary_from_context`  
* `compose.meeting_notes_from_clipboard`  
* `compose.tasks_from_notes`  
* `compose.rewrite(style=…)`

This demonstrates:

* routing  
* multi-app orchestration  
* follow-up refinement  
* permission boundaries  
* trust UI (preview → confirm → write)

---

##### Why this is better than baking it into Notes / Clipboard

If you bake these into Notes:

* Notes becomes bloated  
* extension authors don’t learn the pattern  
* you blur “storage” vs “reasoning”

By keeping Context Composer separate:

* Notes stays a storage/editing app  
* Composer becomes the AI brain  
* third-party devs can copy the pattern

---

## **How users experience it (important)**

Users don’t need to *think* about it most of the time.

They’ll just type:

* “Summarize my last 20 clipboard items”  
* “Turn this into meeting notes”

But advanced users can:

* tag it: `@composer summarize this`  
* inspect its actions  
* disable it if they want

---

### Command {#command}

A **Command** is any natural language input entered by the user into the command bar.

Commands express *intent*, not implementation. A single command may:

* trigger one or more actions,  
* involve one or more applications,  
* require follow-up clarification from the user.

Examples:

* “Create a note from my last clipboard item”  
* “Play my focus playlist on Spotify”  
* “Calculate my rent split and save it to notes”

---

### Tagging {#tagging}

**Tagging** allows users to explicitly constrain or guide command execution by referencing one or more Applications in a command.

Tagging can be used to:

* force execution through a specific Application,  
* constrain execution to specific apps,  
* disambiguate commands.

Tagging acts as a routing constraint, not a guarantee of success. If a tagged Application cannot satisfy the command, Cocommand may request clarification or fail gracefully.

#### Application Tagging (v0)

Users may tag one or more Applications in a command using @.

* If one or more Applications are tagged, Cocommand treats the tagged set as an explicit allowlist.

* During routing and execution, Cocommand may use:  
  * the tagged Applications, and  
  * Kernel Tools (for workspace management and mounting/unmounting tools)  
* Cocommand must not invoke tools from untagged Applications.

If the command cannot be completed using only the tagged Applications, Cocommand requests clarification from the user (e.g., asking to add additional tags or remove the restriction).  
Examples:

* `@calendar add a meeting for 2pm`  
* `@notes @calendar create a meeting note and link it to the 2pm event`

---

### Virtual Workspace (User Concept) {#virtual-workspace-(user-concept)}

The **Virtual Workspace** is a conceptual model that represents the user’s current working context.

It behaves like a virtual window where Applications can be:

* opened,  
* focused,  
* used,  
* and closed.

The Virtual Workspace helps users understand *what Cocommand is currently working with* and *why certain actions are possible* at a given moment.

---

## System Concepts {#system-concepts}

These concepts describe how Cocommand operates internally.

---

### Application Kinds (System)

Internally, Applications are implemented as distinct kinds with different lifecycles and permissions, even if the UX treats them uniformly:

* **SystemApplication**: External OS-backed app (process + window + deep links). Permissions govern OS integration and allowed control surfaces.  
* **BuiltinApplication**: Shipped with Cocommand; owns UI and storage within the app.  
* **ExtensionApplication**: User/third-party app defined by an extension manifest; tools run in the extension host with sandboxing.  
* **OrchestratorApplication**: Headless, AI-native app that provides capabilities but no UI surface; composes outputs from other apps.

System APIs must branch by `app_kind` for lifecycle, permission checks, and tool mounting.

---

### Capability {#capability}

A **Capability** represents a specific action or behavior an Application can perform.

Capabilities are intent-level abstractions such as:

* “Create a note”  
* “Read clipboard history”  
* “Start playback”  
* “Evaluate a calculation”

Capabilities define *what* can be done, independent of how the action is executed.

---

### Capability Router

A system component that maps a user Command to a bounded set of candidate Capabilities/Applications using retrieval (keywords/embeddings) and context (workspace \+ history). The router runs before the LLM planner to reduce complexity and improve reliability.

#### Routing Metadata Contract

Each Application advertises routing metadata for its Capabilities. At minimum:

* `keywords`: short verb/noun triggers (“summarize”, “clipboard”, “meeting notes”)  
* `examples`: realistic user commands for retrieval priming  
* `permissions`: required scopes for the capability (for early gating)  
* `risk_level`: safe / confirm / destructive (for confirmation planning)

The router uses this metadata to shortlist candidates before the LLM planner sees them.

---

### Tool {#tool}

A **Tool** is the executable interface through which a Capability is invoked by the LLM.

Tools define:

* input schema,  
* output schema,  
* side effects,  
* error conditions.

Not all Capabilities must be exposed as Tools. Some Capabilities may be accessible only through UI or other non-LLM interactions.

---

### Kernel Tools {#kernel-tools}

**Kernel Tools** are privileged Tools that can mutate core system state.

Examples include:

* opening or closing Applications,  
* changing focus within the Virtual Workspace,  
* mounting or unmounting Application Tools.

Kernel Tools are always available to the LLM but are restricted by policy and permissions.

---

### Application Tools {#application-tools}

**Application Tools** are Tools exposed by a specific Application.

They become available only when the corresponding Application is active within the Virtual Workspace.

This enables progressive loading and prevents the LLM from being overwhelmed by all possible Tools at once.

---

### Virtual Workspace (System Definition) {#virtual-workspace-(system-definition)}

At a system level, the **Virtual Workspace** is a persistent, structured session state that represents the LLM’s current operating context.

Key properties:

* The Workspace is **LLM-readable** but **not directly writable**.  
* All mutations occur exclusively through Kernel Tools.  
* The Workspace tracks:  
  * active Application instances,  
  * focused Application,  
  * mounted Application Tools,  
  * ephemeral and persistent context relevant to command execution.

The Virtual Workspace acts as the single source of truth for what the LLM can see and do at any point in time.

#### Workspace Schema (Draft)

This is a minimal placeholder for the workspace JSON shape. It should evolve into a formal schema in `docs/` or `SPEC.md`.

```
workspace: {
  workspace_id: string,
  focused_instance_id: string | null,
  instances: [
    {
      instance_id: string,
      app_id: string,
      app_kind: "system" | "builtin" | "extension" | "orchestrator",
      status: "active" | "inactive" | "closed",
      mounted_tool_ids: string[],
      context_refs: string[]
    }
  ],
  context: {
    recent_command_id: string | null,
    follow_up: { ttl_ms: number, turns_remaining: number } | null
  },
  permissions_state: {
    pending_confirmations: string[]
  }
}
```

#### Workspace Contract

* There is at most one focused application instance at a time.  
* Every ApplicationInstance has a stable instance\_id.  
* Only instances with status \= active may have application tools mounted.  
* Mounted application tools must be associated with exactly one instance\_id.  
* Kernel tools are always mounted; application tools are mounted only on demand.  
* Workspace is updated only via Kernel Tools.  
* A failed tool invocation does not mutate workspace state (unless explicitly marked as a partial-safe mutation).  
* Permission-denied operations do not mutate workspace state.  
* Workspace mutations are atomic: either fully applied or not applied.  
* Workspace must be serializable and reloadable without loss of required state.  
* “Close application” is idempotent (closing a closed instance is a no-op).  
* “Open application” returns an instance\_id and is idempotent only when called with a stable dedupe key (optional).

# ---

### LLM Kit {#llm-kit}

The **LLM Kit** is the configuration layer that allows users to select and manage the AI models used by Cocommand.

Users may configure:

* external providers (OpenAI, Anthropic, Google, etc.),  
* model preferences,  
* usage policies and limits.

The LLM Kit is decoupled from Applications and Capabilities, allowing Cocommand to remain model-agnostic.

---

| User Command ↓ Capability Routing (retrieve shortlist) ↓ LLM Planning (choose capabilities \+ steps) ↓ Authorization (permissions/confirmations) ↓ Tool Invocation (system executes) ↓ Workspace Patch \+ Journal Event |
| :---- |

## Permissions & Authorization {#permissions-&-authorization}

Permissions define **what actions are allowed**, **under what conditions**, and **with what level of user consent**.  
 They apply at multiple layers of the system and are enforced independently of the LLM.

Permissions are **declarative**, **auditable**, and **explicitly granted by the user**.

---

### Permission Layer (System Concept) {#permission-layer-(system-concept)}

The **Permission Layer** is a system-level authorization framework that governs:

* Application integration  
* Capability exposure  
* Tool execution  
* Workspace mutation

The Permission Layer operates independently of the LLM and is enforced before any Tool invocation.

---

### Permission Scope {#permission-scope}

Permissions are scoped across three primary levels:

---

### Application Permissions {#application-permissions}

**Application Permissions** define what access an Application requires in order to be installed, enabled, or used within Cocommand.

These permissions are typically requested:

* at installation time,  
* when first activated,  
* or when a new capability is introduced.

Examples:

* Access external APIs (e.g. Spotify playback control)  
* Read or write local files  
* Access network resources  
* Integrate with system-level features

Application Permissions establish the **maximum authority boundary** for all Capabilities and Tools exposed by that Application.

---

### Capability Permissions {#capability-permissions}

**Capability Permissions** define what level of user consent is required for a specific Capability.

Capabilities may be classified by **risk level**:

* **Safe**  
   Read-only or non-destructive actions (e.g. reading clipboard history, listing notes)  
* **Confirm**  
   Actions that modify user data or trigger side effects (e.g. creating or editing notes)  
* **Destructive**  
   Irreversible or high-impact actions (e.g. deleting notes, sending messages, financial actions)

Capability Permissions determine whether:

* execution is automatic,  
* user confirmation is required,  
* or execution is blocked entirely.

---

### Tool Permissions {#tool-permissions}

**Tool Permissions** apply at execution time and represent the final authorization gate before an action occurs.

Tool Permissions:

* validate that Application and Capability permissions are satisfied,  
* enforce contextual constraints (scope, limits, targets),  
* may require explicit user approval.

Tools may fail with authorization errors if permission requirements are not met.

---

### Permission Enforcement Model {#permission-enforcement-model}

* The LLM **cannot grant permissions**.  
* The LLM **cannot bypass permission checks**.  
* All permission decisions are enforced by the system before Tool execution.

If a permission requirement is unmet, the system may:

* request user approval,  
* request clarification,  
* or reject execution.

# ---

### Permission Declarations {#permission-declarations}

Applications and Capabilities declare their required permissions as part of their manifest.

Permissions are:

* explicit,  
* versioned,  
* reviewable by users.

This allows users to understand **what an Application can do** and **why a specific action requires approval**.

---

Permission Interaction with the Virtual Workspace

* The Virtual Workspace reflects only **authorized state**.  
* Unauthorized actions do not mutate workspace state.  
* Permission decisions may result in Workspace transitions (e.g. entering a “confirmation pending” state).

---

### Design Principles {#design-principles}

* Permissions are **layered**, not monolithic.  
* Permissions define **maximum authority**, not intent.  
* The Permission Layer is authoritative; the LLM is advisory.  
* Users remain in control of destructive or sensitive actions.

---

## Trust & Safety UX

Cocommand treats trust as a first-class UX layer, not an optional add-on.  
For any command with risk, ambiguity, or multi-step effects, the UI must make planned actions visible, require consent where needed, and show what changed after execution.

### Action Preview

Before execution, the system may present a concise, human-readable plan (e.g., “Open Notes → Create note → Paste latest clipboard item”).

### Permission-Based Confirmation

Destructive or irreversible actions require explicit user approval based on capability risk level and permission policy.

### Outcome Summaries

After execution, the UI shows a short summary of what changed (e.g., “Meeting created at 2:00 PM”).

### Undo & Action History

Where supported, users can undo recent actions or review a short action history derived from the event stream.

---

## Interaction Model & Session Lifetime

Cocommand is designed as a **command-first interface** with limited, task-scoped conversational continuity.  
 The system prioritizes fast, single-shot execution while supporting short follow-up interactions when appropriate.

---

### Interaction Modes

Cocommand operates in two primary interaction modes:

Command Mode (Default)

* Each user input is treated as an independent command.  
* Commands are interpreted, executed, and completed in a single cycle.  
* No long-term conversational history is retained.  
* This mode is optimized for speed, predictability, and low cognitive overhead.

Examples:

* “Add a meeting to my calendar for 2pm”  
* “Convert 83 USD to EUR”  
* “Create a note from my last clipboard item”

---

#### Follow-Up Mode (Ephemeral)

Follow-Up Mode is automatically entered **after a successful command execution** when the system detects that additional user input may reasonably refer to the previous action.

This mode enables brief, contextual continuations without transitioning into a full chat session.

Follow-Up Mode is triggered when:

* the user submits a new command shortly after completion,  
* the new command contains implicit references (e.g. “it”, “that”, “change this”),  
* or the previous action naturally invites modification or refinement.

Examples:

* “Make it 2:30 instead”  
* “Add Alex to it”  
* “Set it to 45 minutes”

---

### Follow-Up Context Scope

In Follow-Up Mode, Cocommand retains **references**, not conversation history.

The retained context is structured and minimal, and may include:

* the last executed command,  
* a summary of the last action,  
* identifiers of affected entities (e.g. event IDs, note IDs),  
* relevant timestamps or parameters.

No free-form chat transcript is preserved.

---

### Follow-Up Lifetime Policy

Follow-Up Mode is strictly bounded to prevent context accumulation.

Default limits:

* **Time-to-Live (TTL):** 60–120 seconds after last interaction  
* **Maximum follow-up turns:** 1–3 user inputs  
* **Scope:** limited to entities created or modified by the immediately preceding command

Once any limit is exceeded, the system automatically returns to Command Mode.

---

### Session Boundaries

* Closing the command bar ends the **UI interaction**, but does not immediately invalidate an active Follow-Up Mode.  
* If Follow-Up Mode is active and within TTL, reopening the command bar may allow the user to continue the follow-up.  
* If no active follow-up exists, reopening the command bar always starts a new Command Mode interaction.

Workspace state (open applications, focus, pinned apps) persists independently of interaction mode.

---

### Routing Behavior During Follow-Up Mode

While in Follow-Up Mode:

* the routing system strongly biases toward the Application and Capabilities involved in the previous action,  
* unrelated Applications are deprioritized unless the new command clearly indicates a different intent.

If the new command is ambiguous, the system may request clarification.

Examples:

* “Make it 3” → treated as calendar event modification  
* “What’s 17% tip on $83?” → treated as a new command, exiting Follow-Up Mode

---

### Explicit Persistence

Users may explicitly request continued context (e.g. “keep working on this”), which may extend Follow-Up Mode or create a pinned session.  
 Persistent sessions are considered an advanced feature and are not enabled by default.

---

### Design Principles

* Command execution is stateless by default.  
* Context is retained only when it improves usability.  
* Follow-up context is structured, minimal, and time-bound.  
* Workspace state and interaction history are independent concerns.

---

## Execution Latency Model

Cocommand optimizes for **perceived responsiveness** rather than absolute execution time.  
 Different categories of actions have different latency characteristics.

### Latency Classes

| Class | Description | Examples | UX Behavior |
| ----- | ----- | ----- | ----- |
| **Instant** | Purely local, deterministic actions | Calculator, clipboard search, command routing | Immediate result, no loading state |
| **Fast** | Local compute or cached operations | Note creation, formatting, parsing | Inline result, brief feedback |
| **Network-Bound** | Requires external API calls | Calendar, Spotify, email | Progressive feedback, status indicator |
| **Confirmatory** | Requires user approval | Delete, send, modify critical data | Pause \+ confirmation UI |
| **Asynchronous** | Long-running or multi-step tasks | Imports, large syncs | Background execution \+ notification |

---

### Design Guarantees

* The command bar responds immediately to input in all cases.  
* Actions that cannot complete instantly provide visible progress feedback.  
* No action blocks user input while awaiting network or confirmation.  
* Long-running tasks may continue asynchronously after command execution.

---

### Performance Goals (Non-Binding)

Cocommand aims for the following performance targets where feasible:

* Instant / Fast actions: \<200ms perceived response  
* Network-bound actions: immediate acknowledgment \+ progressive updates  
* Confirmatory actions: no execution without explicit user consent

These are goals, not guarantees, and may vary based on environment and provider.  
---

## Mental Model {#mental-model}

* Users think in **Apps & Commands**  
* LLM thinks in **Capabilities**  
* System executes **Tools**  
* Permissions define **what is allowed**  
* Workspace records **what is true**

---

## Extensions & User-Built Applications (Deno Runtime)

Cocommand supports user-built Applications via an extension system. Extensions are authored in TypeScript and executed in a sandboxed Deno Extension Host. This enables a web-friendly developer experience while preserving strong security and deterministic execution through the Rust core.

### Goals

* Enable a large, approachable extension ecosystem (TypeScript-first).  
* Keep the Rust core authoritative for:  
  * permissions and confirmations,  
  * workspace mutations,  
  * event logging / replay,  
  * tool execution orchestration.  
* Ensure extensions are sandboxed, time-bounded, and auditable.

---

### Architecture Overview

Extensions run in a separate process:

* Rust Core (Kernel \+ Router \+ Policy)  
  * Loads extension manifests  
  * Routes commands to candidate capabilities  
  * Authorizes tool calls (permission checks)  
  * Executes tool calls by delegating to Deno  
  * Applies workspace mutations atomically  
  * Writes the event stream and maintains workspace snapshots  
* Deno Extension Host  
  * Loads extension bundles (TypeScript compiled or native TS supported)  
  * Registers tools/capabilities  
  * Runs tool handlers in a constrained runtime  
  * Returns structured results to the Rust core via RPC

### Communication Model

Rust and the Deno host communicate over a local RPC channel (e.g., stdio JSON-RPC, named pipe, or localhost WebSocket).

* Extensions do not directly access privileged OS APIs.  
* All privileged operations must go through Rust-hosted APIs that are permission-gated and logged.

---

### Extension Package Format

An extension is distributed as a package containing:

* manifest.json (required)  
* src/ (TypeScript source) and/or dist/ (bundled output)  
* optional assets (icons, templates)

### Manifest (v0)

The manifest defines metadata, routing hints, permissions, and tool declarations.  
`{`  
  `"extension_version": "0.1",`  
  `"id": "com.example.my-extension",`  
  `"name": "My Extension",`  
  `"description": "Adds custom tools to Cocommand",`  
  `"author": "Example",`  
  `"entrypoint": "dist/index.ts",`

  `"application": {`  
    `"app_id": "my_app",`  
    `"display_name": "My App",`  
    `"app_kind": "extension",`  
    `"ui": { "surface": "none" }`  
  `},`

  `"routing": {`  
    `"keywords": ["ticket", "jira", "issue"],`  
    `"examples": [`  
      `"Create a Jira ticket from these bullets",`  
      `"File an issue for this bug"`  
    `]`  
  `},`

  `"permissions": [`  
    `{ "scope": "network", "level": "ask" },`  
    `{ "scope": "clipboard.read", "level": "allow" }`  
  `],`

  `"tools": [`  
    `{`  
      `"id": "my_app.create_ticket",`  
      `"title": "Create Ticket",`  
      `"risk_level": "confirm",`  
      `"input_schema": { "type": "object", "properties": { "title": { "type": "string" } }, "required": ["title"] },`  
      `"output_schema": { "type": "object", "properties": { "ticket_id": { "type": "string" } }, "required": ["ticket_id"] }`  
    `}`  
  `]`  
`}`

Notes:

* `routing` is used by the **Capability Router** to shortlist candidates.  
* `permissions` declares the maximum authority requested by the extension.  
* `risk_level` participates in permission-based confirmation.

---

### Extension SDK (Tool Registration)

Extensions register tools and implement tool handlers using a Cocommand SDK.

Example `entrypoint`:

`import { defineExtension } from "@cocommand/sdk";`

`export default defineExtension({`  
  `tools: [`  
    `{`  
      `id: "my_app.create_ticket",`  
      `async run(args, ctx) {`  
        `// args validated by Rust core against input_schema`  
        `// ctx provides read-only workspace context and safe references`

        `// Optional: request privileged operations via host APIs:`  
        `// const clip = await ctx.host.clipboard.readLatest();`

        `return { ticket_id: "ABC-123" };`  
      `}`  
    `}`  
  `]`  
`});`

### Execution Contract

* Tool handlers must be pure functions from (args, context) → result whenever possible.  
* Tools must return JSON-serializable outputs matching the declared output\_schema.  
* Tools must not mutate workspace directly; workspace changes occur via Rust kernel tools.

---

### Permissions & Safety

#### Permission Layers

Extensions are governed by the system Permission Layer:

* Application Permissions: requested in the manifest (max authority).  
* Capability / Tool Risk Levels: safe / confirm / destructive.  
* Execution-time Enforcement: Rust core validates permissions before calling Deno.

#### Deno Sandboxing

The Deno host runs with restrictive defaults:

* No filesystem, env, or network access unless explicitly granted by policy.  
* No privileged system calls.  
* Timeouts and resource limits enforced per tool invocation.

All sensitive operations (network, filesystem, integrations) are mediated via Rust-host APIs that:

* enforce permissions,  
* log actions,  
* apply rate limits and scopes.

---

### Lifecycle

1. **Install**  
   * User installs an extension package.  
   * Rust reads manifest, prompts for requested permissions, stores decision.  
2. **Load**  
   * Rust starts/attaches to Deno host (on app start).  
   * Rust instructs host to load enabled extensions.  
   * Host registers tools and returns tool catalog.  
3. **Route**  
   * Router uses routing metadata \+ workspace context to shortlist candidate capabilities.  
4. **Authorize**  
   * Permission Layer checks:  
     * app permissions,  
     * risk level,  
     * user confirmation requirements,  
     * tool budgets.  
5. **Execute**  
   * Rust invokes tool handler in Deno host with validated args.  
   * Deno returns a structured result or error.  
6. **Commit**  
   * Rust records events (tool call request \+ result).  
   * Rust applies any workspace mutations via Kernel Tools (atomic commit/rollback).  
7. **Unload / Disable**  
   * On disable or upgrade, Rust unloads the extension and removes its tools from routing.

---

### Versioning & Compatibility

* Extensions declare extension\_version.  
* The SDK and Host API are versioned independently.  
* Rust core enforces compatibility:  
  * If an extension requires an unsupported SDK/host version, it is not loaded.  
* Tool IDs and manifests should be treated as stable identifiers across upgrades.

---

### Developer Experience (v0)

Recommended workflow:

* `cocommand dev` starts the Deno host in watch mode.  
* Extensions are hot-reloaded on file changes.  
* Tool logs, errors, and timing are visible in a developer console.  
* A validation step checks  
  * manifest schema,  
  * tool schemas,  
  * routing examples,  
  * required permissions.

---

### Design Principles

* TypeScript-first extensions maximize ecosystem growth and iteration speed.  
* Rust core remains authoritative for safety, permissions, and state.  
* Extensions are sandboxed, auditable, and time-bounded.  
* Routing is metadata-driven; tools are mounted progressively via the workspace.

---

# Storage & Persistence (v0)

Cocommand separates storage into an **append-only event log**, **workspace snapshots**, and **domain stores** (clipboard history, settings, extension state).  
In v0, storage is local-only and uses **SQLite** as the default backend for durability and fast queries.

### Event Log (Authoritative History)

* Stores user commands, tool invocations, authorization outcomes, and workspace patches.  
* Redaction is applied by default; sensitive payloads are not stored in raw form.  
* Retention is bounded (e.g., last 5,000 events or last 14 days).

### Workspace Snapshot (Fast Startup)

* Periodic snapshot of the workspace state used for quick rehydration.  
* Snapshots are derived from the event log and do not replace it.

### Recent Actions (Derived)

* The UI “Recent Actions” list is derived from event log summaries.  
* It is not a separate mutable store.

### Clipboard History (Domain Store)

* Clipboard history is stored in-memory only in v0 (bounded ring buffer).  
* By default, only previews and metadata are retained; full content is not persisted.

### Settings & Extension State

* Stored as namespaced key/value data in SQLite.  
* Permissions and LLM provider configuration live here.

---

# Observability & Debugging (v0)

Cocommand includes structured observability to support trust, debugging, and reproducible execution.  
For each tool invocation, the system records:

* tool identifier and caller context,  
* start/end timestamps and duration,  
* authorization outcome (allowed/denied),  
* redacted inputs/outputs plus cryptographic hashes,  
* error codes and sanitized error messages,  
* workspace state hashes before/after mutation (or workspace patch hashes),  
* runtime provenance (model/provider, prompt version, router version, policy profile).

All records are stored in an append-only event stream and may be replayed to rehydrate workspace state.  
 Redaction is applied by default to avoid storing sensitive user content (e.g. clipboard, note bodies, secrets). Development mode may optionally relax redaction for local debugging.  
Cocommand aims to provide:

* a “Recent Actions” history for users (with undo where supported),  
* a developer-facing inspector for extension authors (tool logs, timing, errors),  
* and replay tooling for reproducing failures from recorded events and checkpoints.

---

# Future Enhancements

## Advanced routing & personalization

## Persistent sessions

## Extension marketplace & security reviews

---
