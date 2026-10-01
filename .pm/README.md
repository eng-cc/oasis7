# GitHub Project-Backed PM Operations

Canonical workflow: [capability](../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../doc/engineering/workflow/source-of-truth.md#ready-and-done), [review packet](../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet). The [workflow simplification design](../doc/engineering/workflow/workflow-simplification.design.md) maps the approved requirements to implementation and tests.

This file is an operator command index, not a second workflow specification.

PM 内容规范：[项目管理记录规范](../doc/engineering/doc-governance/project-management-record-standard.design.md)。生命周期、绑定、准入、权限、门禁与收尾仍以 [workflow source of truth](../doc/engineering/workflow/source-of-truth.md) 为准。

## Storage Contract

- GitHub Issues + GitHub Project 是 authoritative project-management truth；GitHub Project 是 active work queue。
- `Task UID` is stable identity. GitHub issue number / Project item id 只是外部对象句柄。
- `.pm/github-project-sync/tasks.json` is the generated task-to-Issue/Project mapping cache; it may be regenerated and is not task truth. Do not edit it manually.
- Task-scoped records, working memory, signals, sessions, stage/gate state, and migration archives are ignored local caches; they are not process truth and must not be committed. Ordinary repository lint must work when these caches are absent.
- Historical operations that need evidence unavailable from complete live GitHub Issue/Project readback must fail closed; a local archive cannot replace GitHub evidence.
- GitHub task issue evidence comments are the formal sink for authorized write-task truth and formal review. Read-only answers and analyses do not create task evidence. Fallback evidence is temporary until replayed.

## Start and Inspect

```bash
./scripts/new-task-worktree.sh <module> <task> --pm-owner-role <role> --pm-title <title> --pm-source-ref <ref>
./scripts/pm/workflow-report.sh --phase start|close|review --role <role>
./scripts/pm/github-project-workflow.sh --json sync
./scripts/pm/github-project-workflow.sh --json audit --task-uid <TASK-UID>
```

`sync` refreshes generated views. `audit` checks selected task/mapping consistency.

`step3-gate` is an explicit historical diagnostic, not a routine lint, task, or PR gate:

```bash
./scripts/pm/github-project-workflow.sh --json step3-gate
```

If its legacy archive is absent, only this diagnostic fails closed; ordinary repository lint and current task/PR operations do not depend on it.

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
./scripts/prepare-task-pr.sh --draft-candidate --create
# after exact-head CI and local role review, record Pre-PR Ready:
./scripts/pm/task-closeout.sh --role <role> --task-uid <TASK-UID> --comparison-ref <ref> \
  --verification-profile <repository-owned-profile> --review-packet-file <canonical-review-packet.json> \
  --ci-ready-receipt <ci_ready_receipt.json>
# after task closeout succeeds:
./scripts/prepare-task-pr.sh --promote-draft <fresh ci_ready_receipt.json>
./scripts/pm/pr-lifecycle-gate.py <pr-number> --json
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

Task-bound legacy `--create` is rejected; promotion requires the fresh receipt and live draft-state checks above.
The canonical links define all lifecycle gates, review attestation, and merge authority. These helpers enforce those definitions; this README does not restate them.

After every planned reviewer has returned, the mechanical v2 review closeout entry is:

```bash
./scripts/pm/review-closeout.sh --task-uid <TASK-UID> --review-plan <plan.json> \
  --role-returns <preflight-ledger.jsonl> --complete
```

It reuses exact handoff/resolution evidence for the unchanged epoch. Finding dispositions must already be explicit; the helper never converts a blocker to `non_actionable`.

For classified non-merge outcomes, follow the [canonical terminal runbook](../doc/engineering/workflow/source-of-truth.md#terminal-runbook).

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
