## Summary

- Collapse local playtest documentation into a `2 + 1` operator-facing menu.
- Demote lower-level launcher bootstrap and A/B sentinel scripts in help text without changing behavior.
- Record task closeout, project trace, and pre-PR local role review evidence.

## Verification

- `git diff --check`
- `bash -n scripts/run-launcher-stack.sh scripts/run-game-test-ab.sh scripts/run-producer-playtest.sh scripts/run-local-letai-game-test.sh`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_b7c7e7e3a3474eb9bf5379fe65c7387c --phase pr-ready`

## PR Evidence

- PR URL: https://github.com/eng-cc/oasis7/pull/441
- task_uid: task_b7c7e7e3a3474eb9bf5379fe65c7387c
