## Summary

- Remove stale p2p/shared-devnet evidence docs after redirecting live references to current evidence.
- Move public-testnet skeleton evidence into templates and keep readiness blocked for template examples.
- Mark older public-testnet live-candidate lane snapshots as archived historical evidence.
- Fix shared-devnet blocker packet empty-array handling under macOS Bash 3.2.

## Verification

- `./scripts/shared-devnet-blocker-packet-smoke.sh`
- `./scripts/network-tier-manifest-smoke.sh`
- `./scripts/network-tier-public-testnet-readiness.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json --lanes-tsv doc/testing/templates/public-testnet-readiness-lanes.example.tsv --out-dir .tmp/task_deab30d8_pr_readiness`
- `./scripts/shared-network-track-gate-smoke.sh`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_deab30d82bd54824b5be64fac1b2c961 --phase pr-ready`
- `git diff --check`

## PR Evidence

- PR URL: https://github.com/eng-cc/oasis7/pull/449
- task_uid: task_deab30d82bd54824b5be64fac1b2c961
- Pre-PR Local Role Review: passed for `repository_health_engineer`, `qa_engineer`, `runtime_engineer`, and `liveops_community`.
- Residual risk: repo-wide `./scripts/pm/lint.sh` still has unrelated historical task-log structure debt outside this task.
