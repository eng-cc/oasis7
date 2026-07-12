# GitHub Project-Backed PM Operations

Canonical workflow: [capability](../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../doc/engineering/workflow/source-of-truth.md#ready-and-done), [review packet](../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet).

This file is an operator command index, not a second workflow specification.

## Storage Contract

- GitHub Issues + GitHub Project 是 authoritative project-management truth；GitHub Project 是 active work queue。
- `Task UID` is stable identity. GitHub issue number / Project item id 只是外部对象句柄。
- `.pm/tasks/` and `.pm/archive/` are generated/cache views; do not edit them manually.
- Task issue evidence comments are the formal evidence sink. Fallback evidence is temporary until replayed.

## Start and Inspect

```bash
./scripts/new-task-worktree.sh <module> <task> --pm-owner-role <role> --pm-title <title> --pm-source-ref <ref>
./scripts/pm/workflow-report.sh --phase start|close|review --role <role>
./scripts/pm/github-project-workflow.sh --json sync
./scripts/pm/github-project-workflow.sh --json audit --task-uid <TASK-UID>
./scripts/pm/github-project-workflow.sh --json step3-gate
```

`sync` refreshes generated views. `audit` checks selected task/mapping consistency. `step3-gate` performs the expensive full-history coverage check.

## Evidence and Execution

```bash
./scripts/pm/append-execution-log.sh ...
./scripts/pm/fallback-evidence.sh create|audit|replay --task-uid <TASK-UID> ...
./scripts/pm/capture-todo.sh --source-ref <path> --summary "<text>"
./scripts/pm/claim-ready.sh --claim-type <claim-type> --verify-command "<command>"
```

- `append-execution-log.sh`: durable step evidence.
- `fallback-evidence.sh`: temporary packet when issue comments are unavailable; replay is mandatory.
- `capture-todo.sh`: reflection intake by default; `--create-task` opts into task creation.
- `claim-ready.sh`: runs one fresh verification command and records its result.

## Pre-PR and PR

```bash
./scripts/pm/task-closeout.sh --role <role> --task-uid <TASK-UID> --comparison-ref <ref> \
  --verification-profile <repository-owned-profile> --review-packet-file <canonical-review-packet.json>
./scripts/prepare-task-pr.sh --create
./scripts/pm/pr-lifecycle-gate.py <pr-number> --json
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

The canonical links define all lifecycle gates, review attestation, and merge authority. These helpers enforce those definitions; this README does not restate them.

If a remote update is partial:

```bash
./scripts/pm/refresh-task-cache.sh --task-uid <TASK-UID> --json
./scripts/pm/github-project-workflow.sh --json audit --task-uid <TASK-UID>
# retry the original helper
```

## Validation

```bash
./scripts/pm/lint.sh
./scripts/pm/workflow-behavior-eval.sh
./scripts/doc-governance-check.sh
```

Implementation entrypoints live under `.agents/skills/`; script-specific `--help` is authoritative for flags.
