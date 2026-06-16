# Slim p2p documentation reading surface

- PR: https://github.com/eng-cc/oasis7/pull/495
- Task UID: task_f638633a5ebe423aac7880485d4b20ca
- Branch: task/engineering-p2p-doc-reading-surface-slimming
- Purpose: normal_pr_ci_watch

## Summary
- Add DistFS and Observer subdomain README entrypoints to fold phase/incremental docs behind master docs.
- Update p2p README/index navigation while preserving exact filename lookup.
- Keep public-testnet/shared-network runbooks and QA evidence landing one-hop reachable with claim-safe wording.

## Verification
- `./scripts/pm/workflow-lint.sh --task-uid task_f638633a5ebe423aac7880485d4b20ca --phase pr-ready`
- `git diff --check`
- Forbidden claim/delete wording scan over changed docs: no matches.

## Residual Risk
- Full `./scripts/doc-governance-check.sh` was interrupted after long no-output scanning and is not used as passing evidence.
