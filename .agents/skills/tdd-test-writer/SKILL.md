---
name: tdd-test-writer
description: Use when a behavior-changing task has a stable automated test surface and needs useful regression or protection tests.
---

# TDD Test Writer

## When to Use

Use this skill when the user asks for test-first work, or when a stable automated
harness can capture a behavior defect or protect an important invariant.

Do not use it as a universal gate for documentation work, behavior-preserving
refactors, or changes whose only verification surface is manual or unstable.

## Oasis7 Test Contract

- For a stable automated behavior defect, keep at least one valid defect-witness
  test that fails against the old behavior and passes after the fix.
- Protective cases that already describe correct behavior may start green. Do
  not break correct behavior just to manufacture RED.
- Syntax, fixture, environment, or harness failures are not defect witnesses.
- Tests may change when a requirement or test assumption was wrong. Preserve the
  diff and reason; do not weaken a valid assertion merely to get green.
- Prefer deterministic, targeted commands. Local GREEN supports development but
  does not replace frozen-candidate verification, independent review, or
  required CI.
- Respect an explicit user request for separate test-first work or a pause.

## Workflow

1. State the behavior, affected paths, and relevant negative case.
2. Reuse the repository's existing test runner and fixture conventions.
3. For a stable bug, add one focused witness and verify its failure is caused
   by the old production behavior. Add initially-green protection cases where
   useful.
4. Continue the same authorized task through implementation and targeted
   verification unless the user or task boundary requires a handoff.
5. Record the changed tests, exact command, result, and any residual gap in the
   existing task truth when the task is write-authorized.

## Guardrails

- Do not weaken behavior assertions merely to make tests pass.
- Do not treat a local test result as formal review or release readiness.
- When no stable automated witness exists, explain the limitation and use the
  strongest available verification without inventing a RED phase.
