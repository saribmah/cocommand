## What each file is for

### `tasks/v0.md` — **Master v0 Milestone Plan**

This is the **single source of truth for what “v0” means**.

**Purpose**

* Defines *all* milestones required for v0
* Shows correct order and dependencies
* Covers **core + UI + platform + extensions**
* Used for planning, tracking, and scope control

**Audience**

* You
* Product planning
* High-level AI orchestration (“what should be built next?”)

**Contains**

* Milestones 0–12
* Cross-cutting concerns
* End-to-end acceptance criteria
* Global Definition of Done

**Example questions this file answers**

* “What does v0 include?”
* “What comes before extensions?”
* “Is follow-up mode part of v0?”
* “Is macOS clipboard support required?”

---

### `tasks/v0-core.md` — **Core / Backend Execution Tasks**

This is a **focused extraction** of only the backend work.

**Purpose**

* Let an AI agent implement the core without seeing UI noise
* Keep Rust work scoped and deterministic
* Prevent the agent from touching frontend or Tauri code

**Audience**

* AI coding agent working on Rust core
* You reviewing backend correctness

**Contains**

* Core milestones only
* Exact file paths in `crates/cocommand`
* No React, no Tauri UI, no CSS
* Backend-specific acceptance criteria and tests

**Example questions this file answers**

* “What Rust files should I modify?”
* “How does routing work?”
* “What does the workspace look like?”
* “What events must be emitted?”

---

### `tasks/v0-ui.md` — **UI / Shell Execution Tasks**

(Complements `v0-core.md`)

**Purpose**

* Let an AI agent focus only on UI behavior
* Prevent backend changes
* Ensure UI follows core contracts

**Audience**

* AI coding agent working on React/Tauri UI

**Contains**

* Command bar UI
* Result cards
* Confirmation flows
* Recent actions UI
* Hotkey + window behavior

---

## How they work together (important)

### Hierarchy

```
v0.md
├── v0-core.md
└── v0-ui.md
```

* `v0.md` defines *what exists*
* `v0-core.md` defines *how the engine works*
* `v0-ui.md` defines *how users interact with it*

**No file should contradict `v0.md`.**

---

## How you should actually use them (recommended workflow)

### When YOU are planning

* Read / edit `v0.md`
* Decide scope changes here first

### When handing work to AI

* Pick **one milestone**
* Choose **one task file**:

    * backend work → `v0-core.md`
    * UI work → `v0-ui.md`
* Use `v0-agent-handoff.md`
* Fill in:

    * milestone ID
    * allowed files
    * task list

### Never give an AI:

* `v0.md` alone (too broad)
* both `v0-core.md` and `v0-ui.md` at once (scope bleed)

---

## Why this separation is crucial for AI-driven development

Without this split, AI will:

* jump ahead
* “helpfully” implement UI while working on backend
* invent APIs
* touch files it shouldn’t

With this split:

* tasks are deterministic
* diffs are reviewable
* failures are localized
* you can parallelize safely

---

## TL;DR mental model

* **`v0.md`** → *What v0 means*
* **`v0-core.md`** → *Build the engine*
* **`v0-ui.md`** → *Build the shell*
* **`v0-agent-handoff.md`** → *How to delegate safely*

You’ve basically created an **AI-native software development workflow** here — this is exactly how you scale without losing control.
