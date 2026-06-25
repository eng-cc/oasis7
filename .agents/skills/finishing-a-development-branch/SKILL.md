---
name: finishing-a-development-branch
description: "Use when implementation is done and the work needs final verification, pre-PR local role review, closeout, commit, PR creation, review handling, merge, and worktree cleanup."
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.


# Finishing a Development Branch

## When to Use

Use this skill when code and docs are already updated and you are moving into branch closure:

- close the task
- verify the final diff
- collect fresh local involved-role subagent review
- close the task after review findings are resolved
- commit
- prepare or create the PR
- decide whether the PR is a normal CI/review PR or a manual packaging CI hold
- watch required checks/review and merge normal PRs
- handle review comments
- clean up after merge

## Default Oasis7 Path

1. Confirm the task has its own worktree and `.pm` task.
2. Run the final local checks for the changed surface.
3. Dispatch fresh local subagent review for every involved relevant role, address valid findings, and record the passed evidence packet before final closeout or commit.

Use the same role-selection rule as `requesting-repo-owned-review`, including:
- `producer_system_designer` when scope, product contract, user promise, acceptance, or system-level semantics change
- `gameplay_designer` when gameplay rules, progression, balance, encounter/resource loops, or player verb semantics are touched
- `game_visual_interaction_designer` when visible UI/gameplay presentation, visual direction, interaction feel, player-facing screen flow, screenshot/visual-review surfaces, accessibility/readability, or UI-heavy claims are touched
- `runtime_engineer` when runtime/server/simulation/gameplay enforcement, replay, recovery, checkpoint, long-run behavior, or `crates/oasis7*` runtime paths are touched
- `blockchain_ops_engineer` when deployment, node ops, topology/inventory, service/host contracts, health baselines, upgrade/rollback/restore drills, packaging/release ops, or operator-facing runbooks are touched
- `wasm_platform_engineer` when `crates/oasis7_wasm_*`, builtin wasm modules, ABI/schema, manifest/hash, wasm build/receipt, wasm determinism workflows, or `doc/world-runtime/wasm/*` are touched
- `agent_engineer` when agent behavior, prompts, provider contracts, model/runtime config, subagent dispatch contracts, or agent tooling are touched
- `viewer_engineer` when Viewer/Web/UI/WebGPU/browser validation paths are touched
- `qa_engineer` when the claim depends on verification, release readiness, test strategy, or evidence sufficiency
- `repository_health_engineer` when the diff changes cross-cutting architecture, shared workflow surfaces, docs/code contracts, large refactors, repeated bug signatures, or known technical-debt boundaries
- `liveops_community` when external messaging, incidents, player promises, community feedback, release notes, or channel runbooks are touched

4. Close the task:

```bash
./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> --verify-command "<fresh verification command>"
```

5. Commit exactly this task slice.

The pre-PR local role review packet is recorded before final closeout/commit and uses this shape:

```markdown
- Pre-PR Local Role Review: passed
- Task UID: <task_uid>
- Source Worktree: <absolute path>
- Source Branch: <branch>
- Source Head: <reviewed git sha; must be current source head or an ancestor whose later changes are only the task review evidence files>
- Comparison Ref: <base ref>
- Reviewed Changed Paths: <semicolon-separated paths or diff summary ref>
- Review Package: <path to review package or n/a with reason>
- Role Selection Basis: <changed paths + task slice history + explicit includes/skips>
- Review Roles: <comma-separated roles>
- Review Evidence: <per-role section or handoff refs>
- Review Verdicts: <per-role scope/spec compliance verdict + role quality/risk verdict>
- Review Findings Disposition: <addressed | no_findings>
- Finding Disposition Evidence: <fix refs or rejected/stale evidence refs>
- Verification Matrix: <changed surface -> required evidence -> observed evidence or explicit deferral>
- Visual Evidence: <screenshot/model visual review paths or n/a with exemption reason>
- WASM Evidence: <support crate/determinism evidence or n/a with reason>
- Ops Evidence: <readiness/rollback/runbook/operator evidence or n/a with reason>
- LiveOps Evidence: <messaging/release-note/status/community evidence or n/a with reason>
- Residual Risk: <text>
- Slice Ledger: <path to slice ledger or n/a with reason>
```

6. Run PR preflight / create:

```bash
./scripts/prepare-task-pr.sh --create
```

7. Record the PR purpose decision:
   - `normal_pr_ci_watch`: default for ordinary implementation/documentation PRs. Keep watching required checks, mergeability, review decisions, comments, and unresolved review threads. Treat `REVIEW_REQUIRED` as informational, not as a blocker. Treat `mergeStateStatus=BEHIND` as informational too unless GitHub actually requires a branch update or reports a conflict; if the PR is otherwise mergeable and the repository merge path accepts it, merge can proceed without a local rebase. If `mergeStateStatus=BLOCKED` is only missing review approval and user/task policy explicitly allows skipping it, use the repository admin merge path as normal flow after re-checking gates.
   - `manual_packaging_ci_hold`: only when the user/task says the PR exists specifically to run manual-trigger packaging/release CI jobs. Record the manual job/purpose, responsible operator/role, expected success signal, stale-date/timeout escalation, ops readiness/rollback/runbook evidence when release ops are implicated, external/status messaging evidence when player- or community-facing, and the exact resume criterion. Stop before auto-merge until the operator/user resumes.

8. For `normal_pr_ci_watch`, keep the loop moving without waiting for another prompt:
   - if checks fail, inspect the failing job, fix, rerun local verification, push, and continue watching
   - before merge, explicitly check PR comments and review threads; if review comments arrive, use:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

   - when required checks pass, the PR is mergeable through the repository/GitHub merge path or through an explicitly authorized review-approval admin path, PR comments/review threads have been checked, and no requested changes, actionable comments, or blocking unresolved review threads remain, merge the PR using the repository's configured merge path

9. After merge, sync local `main` and remove the task worktree / branch.

## Required Checks Before Commit

- worktree diff matches task scope
- task execution log updated
- relevant formal docs updated
- local verification rerun for the affected surface
- pre-PR local role review packet recorded when the next step is PR creation
- PR purpose decision recorded after PR creation
- normal PRs are watched through required checks, mergeability, review decisions, and comment closeout to merge; `REVIEW_REQUIRED` alone is not a blocker, `BEHIND` alone does not force a local rebase when the repository merge path can still merge cleanly, and review-approval-only `BLOCKED` may use an explicitly authorized admin merge path; only manual packaging CI PRs may pause before merge

## Post-Merge Cleanup

- fast-forward local `main`
- remove the task worktree
- delete the task branch after leaving that worktree

## Known Failure Modes

- Stopping at PR creation for a normal PR instead of continuing through checks, comments, mergeability, merge, and cleanup.
- Treating `REVIEW_REQUIRED` or `BEHIND` as automatic blockers when repo policy says they are informational by themselves.
- Merging before checking unresolved comments, review threads, requested changes, and required checks.
- Deleting or cleaning up the worktree before the merge and main-branch sync are actually complete.

## Guardrails

- Do not land locally unless the user explicitly asks for local landing.
- Do not skip `.pm` closeout just because the execution log is updated.
- Do not claim "done" while the branch still lacks required verification or PR creation.
- Do not treat review-thread resolution as merge readiness.
- Do not stop at PR creation for normal PRs; continue watching CI/review, fix failures, merge, and clean up.
- Do not merge a normal PR without first checking PR comments and review threads and resolving or answering actionable items.
- Do not auto-merge PRs opened specifically for manual packaging/release CI until the operator/user resumes the normal merge path.
