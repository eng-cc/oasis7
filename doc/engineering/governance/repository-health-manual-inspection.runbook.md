# Repository Health Manual Inspection Runbook

## Purpose
This runbook defines the manually triggered repository-health inspection used by engineering governance owners.

The inspection is a human-triaged health review. It does not add a GitHub Actions hard gate, and it does not replace the normal task/worktree, professional slice, verification, PR, or merge workflow.

## Trigger

The inspection is started manually by the engineering governance owner. There is no `cc-connect`, cron, GitHub Actions schedule, or other automatic trigger for this workflow.

When the owner starts an inspection, still enter the standard `oasis7` workflow: create or enter one task worktree, bind one `.pm` task, and dispatch `repository_health_engineer` for professional repository-health judgment.

## Checklist
Run the checks from the task worktree and record the command outputs or summaries in `.pm/tasks/<TASK-UID>.execution.md`.

```bash
./scripts/doc-inventory-report.sh
./scripts/doc-governance-check.sh
./scripts/lint-skills.sh
./scripts/worktree-gc-report.sh --prunable-only
./scripts/pm/lint.sh
```

## Interpretation
- `doc-governance-check` failure: treat as P0/P1 engineering-governance follow-up candidate.
- `lint-skills` failure: treat as P0/P1 workflow-surface follow-up candidate.
- `doc-inventory-report` `action_required`: classify by module or hotspot path, then decide whether to create a focused path-governance task or leave as quarterly trend evidence.
- `worktree-gc-report --prunable-only`: read-only only. Do not copy cleanup commands blindly; confirm the reported worktree is not the main worktree, has no useful dirty state, and is not part of an active task.
- `pm lint` failure: classify current-task failures separately from known historical execution-log evidence debt. Do not make historical debt a blocking inspection finding unless a follow-up task has scoped it.

## Evidence Sink
The canonical inspection evidence is the `.pm` task execution log:

```text
.pm/tasks/<TASK-UID>.execution.md
```

The chat summary should only report the role-attributed findings, follow-up candidates, and residual risk. It is not a replacement for the execution log.

High-value but not-yet-owned follow-up work should first be captured as a reflection signal:

```bash
./scripts/pm/capture-todo.sh --source-ref <path> --summary "<finding summary>"
```

Promote a signal to a formal task only after the owner chooses to create the follow-up.

## Escalation
- Repository-health, documentation/code alignment, semantic clarity, workflow drift, task evidence debt: keep with `repository_health_engineer`.
- Verification sufficiency or release blocking judgment: dispatch `qa_engineer`.
- Runtime, Viewer/Web, WASM, agent, blockchain ops, gameplay, or visual/interaction findings: dispatch the matching professional role before presenting a domain conclusion.
- External-facing community, incident, or player promise wording: dispatch `liveops_community`.

## Quarterly Review
When the owner chooses to run a quarterly review, compare recent manual inspection trends:
- repeated `doc-inventory-report` hotspots
- repeated worktree cleanup candidates
- `pm lint` historical-debt burn-down or growth
- recurring `doc-governance-check` or `lint-skills` failure signatures
- whether any reflection signals should become committed `.pm` tasks

Quarterly conclusions should update the relevant engineering governance project/topic docs only when they change policy, thresholds, or active follow-up ownership.
