# Add small-player lane runtime truth

- Task UID: task_96b6823495f44ef39c80f3c8b1a74421
- PR URL: https://github.com/eng-cc/oasis7/pull/573
- Source Branch: task/game-small-player-lane-runtime-truth
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Add canonical small-player lane runtime truth fields and legacy backfill for anti-grind checks.
- Derive grind-only and forced major-power dependency state in runtime_live gameplay snapshots.
- Add focused positive regressions for grind-only and forced-dependency surfaces.
- Record current game and engineering project traces without claiming mature-world QA lane pass.

## Verification
- `./scripts/ci-tests.sh required`
- `./scripts/pm/workflow-lint.sh --task-uid task_96b6823495f44ef39c80f3c8b1a74421 --phase pr-ready`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## Local Role Review
- gameplay_designer: no findings; gameplay design scope pass.
- runtime_engineer: no findings; prior grind-only coverage finding satisfied.
- viewer_engineer: no findings; viewer/API compatibility pass.
- qa_engineer: no findings; PR creation gate pass.
- repository_health_engineer: no findings; package/diff hygiene pass.

## Residual Risk
- This PR establishes runtime/sample truth and focused regressions; it does not claim mature-world small-player QA lane pass.
- A later QA mature-world sample/verdict should decide pass/watch/block using the new fields.
