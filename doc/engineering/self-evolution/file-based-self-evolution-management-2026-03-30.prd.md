# oasis7: self-evolution file-based PM historical background (2026-03-30)

- Design background: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- Current workflow truth: `doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract`

Audit round: 8

## Status

This file is a historical pointer for the March 2026 `.pm` file-based project
management rollout. It is no longer an active task-truth or execution-evidence
specification.

Current rules:

- GitHub Issues and GitHub Project items are the task collaboration and queue
  truth.
- Execution evidence is written to GitHub task issue evidence comments.
- `.pm/github-project-sync/tasks.json` is a generated task mapping cache.
- `.pm/github-project-sync/task-archive.jsonl` is an audit bridge for historical
  task metadata and evidence.
- Repo-local `.pm` surfaces may still hold role memory, task-scoped
  `working_memory`, stage/gate state, generated views, and migration archives
  only as defined by the workflow source of truth.

Retired March 2026 terms in the original rollout included
`.pm/tasks/<task_uid>.yaml`, `.pm/tasks/<task_uid>.execution.md`,
`.pm/inbox/signals.jsonl`, task registry files, and role backlog files as
current task truth. Those descriptions must not be copied into new docs,
operator guidance, task evidence, or PR-readiness packets.

## Historical Scope

The original topic introduced these repo-local ideas:

- role memory and superseded memory chains
- task-scoped working memory
- reflection intake
- stage/gate current-state files
- generated role backlog and task views
- PM helper scripts for local reporting and migration support

These concepts remain valid only where the current workflow source of truth
keeps them. When this historical file conflicts with
`doc/engineering/workflow/source-of-truth.md`, the workflow source of truth wins.

## Current Reading Path

- Current task/workflow contract:
  `doc/engineering/workflow/source-of-truth.md`
- Current object-model background that still matters for repo-local memory and
  stage/gate surfaces:
  `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- Long-term role memory:
  `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- Memory/working-memory follow-up:
  `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`

## Preserved Traceability

This file keeps the `PRD-ENGINEERING-021` historical anchor for older task rows
and scripts that still cite the self-evolution file-based PM rollout. It does
not define active acceptance criteria, current task evidence sinks, or a project
execution plan.
