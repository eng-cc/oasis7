---
name: synchronization-algorithms
version: "2.0.0"
description: Use when an oasis7 runtime slice needs to analyze or design authority, state consistency, reconciliation, prediction, interpolation, lag handling, or multiplayer synchronization behavior.
sasmp_version: "1.3.0"
bonded_agent: 05-networking-multiplayer
bond_type: PRIMARY_BOND
---

# Synchronization Algorithms

## Oasis7 Workflow Binding

In oasis7, this is a runtime domain skill for state consistency and authority
semantics. It is domain-triggered and non-default; it does not replace
`runtime_engineer` ownership of runtime correctness.

Use it only inside the bound GitHub-backed task/worktree and tie conclusions to current
runtime code, tests, design docs, traces, or playtest evidence.

## When to Use

Use this skill when:

- a task touches authoritative state, client/server reconciliation, prediction,
  interpolation, lag handling, rollback, or desync recovery
- `runtime_engineer` needs reusable synchronization framing for a current
  oasis7 implementation or design decision
- networked gameplay behavior must be described in terms of consistency,
  latency, and player-visible correction

Do not use this skill when:

- the task is generic networking education without an oasis7 runtime decision
- the issue is blockchain/node operations rather than game/runtime state
  consistency
- the conclusion has not been routed through `runtime_engineer`

## Core Workflow

1. Identify the authority boundary: which component owns truth, what state is
   derived, and what can be predicted locally.
2. Ground the analysis in current repo evidence: runtime code paths, tests,
   logs, PRD/project docs, or playtest traces.
3. Choose the narrow synchronization lens needed for the task: reconciliation,
   prediction, interpolation, lag compensation, rollback, or resync.
4. Define a verification surface before recommending a behavior change.

## Supporting Files

- `references/SYNC_GUIDE.md`: compact upstream-tracking reference for generic
  synchronization terms. Read it only when the runtime slice needs a refresher.

## Known Failure Modes

- Copying generic multiplayer patterns without checking oasis7 authority and
  deterministic-state boundaries.
- Treating smooth visuals as state correctness.
- Fixing latency symptoms without a replayable or observable desync check.

## Guardrails

- Keep synchronization advice bound to current oasis7 runtime authority,
  state-consistency, and task evidence.
- Do not use this skill for blockchain/node operations, generic networking
  education, or implementation claims outside `runtime_engineer` ownership.
- Do not recommend a synchronization pattern without a focused verification
  surface or documented residual risk.

## Verification

- Run the focused runtime test, trace comparison, or playtest probe tied to the
  state-consistency decision.
- Record the authority boundary, evidence, and residual risk in the task log.
- Run `./scripts/lint-skills.sh` after skill-surface edits.
