---
name: verification-before-completion
description: Use when about to claim a task is complete, tests passed, a branch is ready for PR, or a PR is ready to merge. Requires fresh verification, direct output inspection, and use of `./scripts/pm/claim-ready.sh` when the claim can be mapped to one verification command.
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.

> PM truth: if the claim depends on PM queue/status state, include `./scripts/pm/github-project-workflow.sh ... audit` as a fresh verification input.


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

A fresh verification claim must come from the current verification epoch: after
the latest code/doc/script change that can affect the claim, after any valid
review finding fix, and after any branch sync that changes the reviewed diff.
Earlier successful output is background only.

## Preferred Workflow

1. Choose the claim type:
   - `task_complete`
   - `tests_passed`
   - `ready_for_pr`
   - `ready_for_merge`
2. For `ready_for_pr`, use the trusted same-head CI receipt; other claims choose a matching command/profile.
3. Prefer the repo helper:

```bash
./scripts/pm/claim-ready.sh \
  --claim-type ready_for_pr \
  --verification-profile repository_required \
  --ci-ready-receipt <receipt.json>
```

4. Read both the command output and the exit status.
   When verification output is broad, use `./scripts/pm/bounded-command-output.py` and cite both its bounded summary and full artifact digest; truncation must remain explicit.
5. Only make the claim if the verification succeeded in the current run.

## When the Helper Is Not Enough

If the claim depends on multiple checks, run each check explicitly and summarize the actual status:

- what ran
- what passed
- what failed
- what remains blocked

Do not collapse multi-check state into a blanket "ready" claim unless every required check passed.

## Output Rules

- Cite the exact command you ran.
- Say whether the result is from a fresh run.
- If blocked, lead with the blocker instead of a progress summary.
- Distinguish local verification from GitHub review / required checks.

## Oasis7-Specific Checks

- Current task Doc / PM work: `./scripts/pm/workflow-lint.sh --task-uid <TASK-UID> --phase current`, `./scripts/doc-governance-check.sh`, `git diff --check`
- Repo-wide PM governance: `./scripts/pm/lint.sh`
- Task closeout readiness: `./scripts/pm/task-closeout.sh --role <role> --task-uid <TASK-UID> --verification-profile repository_required --review-packet-file <canonical-review-packet.json> --ci-ready-receipt <receipt.json>`
- PR readiness: passed pre-PR local role review packet in GitHub task issue evidence comments, then `./scripts/prepare-task-pr.sh`

## Guardrails

- Never claim success from expected behavior alone.
- Never infer "merge ready" from "thread resolved".
- Never say tests passed unless you ran the relevant test command in this turn.

## Known Failure Modes

- Reusing yesterday's successful output or a previous agent's summary as fresh verification; rerun or clearly report that verification is stale.
- Collapsing multiple required checks into one success claim when only one command ran; list each passed, failed, or blocked check.
- Treating "no visible error" as test success without inspecting the command exit status and relevant output.
- Calling a branch ready for PR before the pre-PR local role review packet is present when that gate applies.
