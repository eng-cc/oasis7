---
name: run-product-loop
description: Use when the user explicitly requests a bounded product loop task or continuation of that task.
---

# Run product Loop

Follow [canonical manual entry](../../../doc/engineering/workflow/source-of-truth.md#manual-three-loop-transition) and [PM truth](../../../doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract).

## When to Use

Process only this user request. If it contains no task or bounded objective, obtain the missing objective before writes. A skill match or Issue comment is not authorization.

1. Use default-workflow-bootstrap for the one canonical task/worktree; bind loop `product`, manual request reference, fixed base, immutable inputs and owner through the PM adapter. Preserve legacy tasks unless the user explicitly requests migration with a new bootstrap epoch.
2. Run `python3 scripts/pm/loop.py doctor --repo-root <worktree> --tool-root <effective-tool-worktree> --task-uid <UID> --json`. Effective helper bytes must match the task's immutable policy commit on the trusted default branch. Activation prerequisites block; candidate helpers cannot grant permission.
3. For continuation run `loop.py resume-check` with the same identity and `--manual-request-ref <current-request>`. Resolve reported blockers using existing journals; never replay uncertain side effects.
4. Route through repo-owned-workflow-router and professional slices. Keep scope/contracts, same-head verification, review, CI and merge holds intact. Final diff compliance is not filesystem isolation proof.
5. On completion or stable wait record resumable evidence and return. Do not register heartbeat, schedule a wakeup, recurse into another loop, or start the next task. Only a new user continuation resumes this task.

## Guardrails

Validate the helper with `python3 scripts/pm/loop.test.py`; this fixture evidence does not prove native client isolation or live activation.
