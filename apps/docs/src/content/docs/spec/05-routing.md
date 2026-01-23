---
title: Routing
---

Routing reduces tool overload by narrowing the candidate set before planning.
The router operates on metadata and context, not on full tool catalogs.

## Routing Goals

- High recall with bounded candidate size.
- Deterministic, debuggable selection.
- Context-aware prioritization.

## Routing Inputs

- User command text.
- Workspace context (focused app, recent apps, pinned apps).
- Tagging constraints (@app).
- Capability metadata (keywords, examples).

## Two-Stage Routing

### Stage A: Candidate Generation
- Lexical search over keywords and examples.
- Embedding search over example commands.
- Merge and dedupe top candidates.

### Stage B: Reranking
- Lightweight LLM rerank or rule-based scoring.
- Output a bounded set of 3–7 apps/capabilities.

## Routing Output

- Capability IDs (preferred)
- Derived Application IDs
- Confidence score and explanation

## Tagging Behavior

- If user tags apps, the tagged set is an explicit allowlist.
- Router must not select untagged apps.
- If the command cannot be satisfied, request clarification.
