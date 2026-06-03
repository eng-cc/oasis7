# task_8c6cadfbece54baf944ecc9190606c5a Execution Log

- task_uid: task_8c6cadfbece54baf944ecc9190606c5a
- title: fix v0.0.56 release package failures
- owner_role: tpm
- worktree_hint: /Users/scc/ccwork/worktrees/oasis7-site-trigger-latest-github-release

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
  - Action: ...
  - Validation Command: ...
  - Expected Result: ...
  - Actual Result: ...
  - Blocker / Next Action: ...
-->

## 2026-06-03 15:03:42 CST / tpm
- 完成内容: Investigated and patched the failed `v0.0.56` Release Packages rerun.
- 遗留事项: Needs commit/push to PR #342 and a rerun of `release-packages.yml` with `tag=v0.0.56` after the PR branch update is available; S9 failure on run `26867434448` was observed on `main` checkout, not on the actual `v0.0.56` tag checkout.
- Action: Inspected `gh run view 26867434448`, failed job logs/artifacts for `release-gate-soak` and `release-gate-web`, the PR branch workflow run `26868674338`, and the workflow checkout configuration.
- Validation Command: `gh run view 26867434448 --json databaseId,status,conclusion,url,workflowName,createdAt,updatedAt,headBranch,headSha,jobs`; `gh run view 26867434448 --job 79234213378 --log`; `gh run view 26867434448 --job 79234213391 --log`; `gh run download 26867434448 --name release-gate-soak-summary --dir .tmp/gh-run-26867434448/soak-artifact`; `gh run download 26867434448 --name release-gate-web-summary --dir .tmp/gh-run-26867434448/web-artifact`.
- Expected Result: Identify whether the failed rerun really built `v0.0.56`, then identify actionable failure signatures.
- Actual Result: Run `26867434448` was `workflow_dispatch` with `tag=v0.0.56` intent but checked out `main` (`headSha=770dee5b6dc417051fac8c3c7011474b91af20cb`) rather than the `v0.0.56` tag SHA (`4eb0fe9099aa91af408e5bb41eddd52d3af3ee0f`). `release-gate-soak` failed S9 after pause chaos (`consensus_hash_samples_missing`, `settlement_apply_attempts_zero`, `known_peer_heads_zero_samples=111`). `release-gate-web` failed because the script saw no logical/event progress and did not recognize the current blocked fields, even though artifacts showed `gameplaySummary.executionState=blocked` and `gameplaySummary.recentFeedback.stage=blocked` for provider unreachable.
- Action: Updated `.github/workflows/release-packages.yml` so every `actions/checkout@v6` uses `RELEASE_CHECKOUT_REF`, resolving to `inputs.tag` for `workflow_dispatch` and `github.ref` for tag pushes. Updated `scripts/viewer-software-safe-step-regression.sh` to recognize explicit blockers from `gameplaySummary.executionState`, `gameplaySummary.recentFeedback.stage`, and `gameplaySummary.narrativeBlockerDetail`, and replaced the macOS-incompatible `${AGENT_ID@Q}` with Python JSON quoting. Updated the smoke fixture to cover the current provider-unreachable blocked field shape.
- Validation Command: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-packages.yml"); puts "workflow yaml parsed"' && bash -n scripts/viewer-software-safe-step-regression.sh scripts/viewer-software-safe-step-regression-smoke.sh && ./scripts/viewer-software-safe-step-regression-smoke.sh && git diff --check`.
- Expected Result: Workflow YAML parses, shell scripts parse on local macOS bash, the smoke accepts the current blocked state shape, and the diff has no whitespace errors.
- Actual Result: Workflow YAML parsed; `viewer software_safe step regression smoke checks passed`; `git diff --check` passed.
- Blocker / Next Action: Close out this task, commit/push the PR branch update, then rerun/monitor the package workflow.
