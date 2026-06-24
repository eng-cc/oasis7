---
name: verification-before-completion
description: Use when about to claim a task is complete, tests passed, a branch is ready for PR, or a PR is ready to merge. Requires fresh verification, direct output inspection, and use of `./scripts/pm/claim-ready.sh` when the claim can be mapped to one verification command.
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.


# Verification Before Completion

## When to Use

Use this skill before saying any of the following:

- the task is complete
- tests passed
- the branch is ready for PR
- the PR is ready to merge

Do not use stale output, partial output, or earlier successful runs as proof.

## Core Rule

Run the verification command now, read the result now, and only then make the claim.

If the command fails, report the blocked state directly. Do not soften it into a success summary.

## Preferred Workflow

1. Choose the claim type:
   - `task_complete`
   - `tests_passed`
   - `ready_for_pr`
   - `ready_for_merge`
2. Choose one concrete verification command that matches the claim.
3. Prefer the repo helper:

```bash
./scripts/pm/claim-ready.sh \
  --claim-type ready_for_pr \
  --verify-command "./scripts/doc-governance-check.sh"
```

4. Read both the command output and the exit status.
5. Only make the claim if the verification succeeded in the current run.

## When the Helper Is Not Enough

If the claim depends on multiple checks, run each check explicitly and summarize the actual status:

- what ran
- what passed
- what failed
- what remains blocked

For parent initiative or domain child task claims, also distinguish:

- `module_required`: module-local fast-loop evidence
- `module_full`: risk-triggered broader module evidence, or why it was not triggered
- `integration_required`: cross-module evidence, or the explicit checkpoint/owner that defers it
- `release_full`: release confidence evidence, or the explicit non-release scope

Do not call a module-local pass release-ready unless the integration/release gates are also satisfied or explicitly deferred with an owner and resume criterion.

Do not collapse multi-check state into a blanket "ready" claim unless every required check passed.

## Output Rules

- Cite the exact command you ran.
- Say whether the result is from a fresh run.
- If blocked, lead with the blocker instead of a progress summary.
- Distinguish local verification from GitHub review / required checks.

## Oasis7-Specific Checks

- Current task Doc / PM work: `./scripts/pm/workflow-lint.sh --task-uid <TASK-UID> --phase current`, `./scripts/doc-governance-check.sh`, `git diff --check`
- Repo-wide PM governance: `./scripts/pm/lint.sh`
- Task closeout readiness: `./scripts/pm/task-closeout.sh --role <role> --task-uid <TASK-UID> --verify-command "<fresh verification command>"`
- PR readiness: passed pre-PR local role review packet in `.pm/tasks/<TASK-UID>.execution.md`, then `./scripts/prepare-task-pr.sh`
- Parent/child workflow readiness: parent initiative packet, child task packet, dependency contracts, verification contract matrix, and mock/fixture proof boundaries where local substitutes were used

## Guardrails

- Never claim success from expected behavior alone.
- Never infer "merge ready" from "thread resolved".
- Never say tests passed unless you ran the relevant test command in this turn.

## Known Failure Modes

- Reusing yesterday's successful output or a previous agent's summary as fresh verification; rerun or clearly report that verification is stale.
- Collapsing multiple required checks into one success claim when only one command ran; list each passed, failed, or blocked check.
- Treating "no visible error" as test success without inspecting the command exit status and relevant output.
- Calling a branch ready for PR before the pre-PR local role review packet is present when that gate applies.
