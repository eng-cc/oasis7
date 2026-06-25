# task_22e2decb01b04e7d9cc9f94caecbb308 Execution Log

- task_uid: task_22e2decb01b04e7d9cc9f94caecbb308
- title: fail rebuild cleanup when post-cleanup orphan reappears
- owner_role: tpm
- worktree_hint: /Users/scc/ccwork/worktrees/oasis7-p2p-testnet-rebuild-cleanup-post-success-orphan

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

## 2026-06-25 08:49:34 CST / tpm
- 完成内容: WORKFLOW BOOTSTRAP DECIDED for post-PR #627 live rebuild failure. Repository state impact: yes, rebuild cleanup script/tests/docs may need another fix because live evidence shows cleanup still returns success while a sequencer stack-root orphan reappears. Isolation decision: created dedicated worktree `/Users/scc/ccwork/worktrees/oasis7-p2p-testnet-rebuild-cleanup-post-success-orphan` on branch `task/p2p-testnet-rebuild-cleanup-post-success-orphan`. Task truth: `task_22e2decb01b04e7d9cc9f94caecbb308`, owner role `tpm` as workflow coordinator only.
- Evidence: after PR #627 merged and local main fast-forwarded to `65632e37ffd92b823cab5606d6544566298d3664`, live command `./scripts/p2p-public-testnet-rebuild-validators.sh ... --out-dir .tmp/public-testnet-update-20260624T126/rebuild-validators-live-signers-65-858-after-627` exited 1 with `sequencer readiness failed checks after restart`. Captured status: `failed_gates=["consensus_peer_head_unavailable","replication_recent_errors"]`, `known_peer_heads=0`, `connected_peer_count=3`, `recent_replication_error_count=270`. Remote post-failure sample: sequencer systemd `inactive`, but `/opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime` pid `2521777` listened on `6631/6831` under `bash /opt/oasis7/p2p-testnet/bin/start-node.sh` pid `2521768`; storage systemd inactive with no matching port/process.
- Reproduced failure: peer-head/readiness failures remain downstream of incomplete rebuild cleanup. The new code quiesces systemd inside the cleanup loop, but cleanup can still return success before a post-cleanup orphan reappears from service wrapper/systemd restart timing.
- WORKFLOW ROUTE DECIDED: selected `systematic-debugging` for root-cause narrowing and patch; use fake ssh/systemd regression if stable; then `verification-before-completion`, local role review, PR, CI, merge, and live rebuild.
- Subagent Slice Plan:
  - role: `blockchain_ops_engineer`
  - slice type: ops cleanup root-cause review
  - intended model configuration: workflow default subagent runtime
  - actual dispatched model/reasoning: pending/inherited-unverified
  - context delivery mode: full-thread/full-history fork plus this task log
  - mandatory context checklist/packet: AGENTS workflow; role card; task uid/path; live evidence above; changed files likely rebuild script/test/doc; user root-cause-first principle
  - write scope: no direct write expected; review/hypothesis output only
  - return contract: confirm/challenge root cause, required cleanup semantics, deployment safety concerns, residual risk
  - formal sink / writeback surface: this execution log
  - integration owner/order: tpm integrates before PR
  - context exemption: none
- Subagent Slice Plan:
  - role: `qa_engineer`
  - slice type: regression false-pass review
  - intended model configuration: workflow default subagent runtime
  - actual dispatched model/reasoning: pending/inherited-unverified
  - context delivery mode: full-thread/full-history fork plus this task log
  - mandatory context checklist/packet: AGENTS workflow; role card; task uid/path; previous fake systemd tests; live evidence above
  - write scope: no direct write expected; review/hypothesis output only
  - return contract: identify regression that proves post-cleanup orphan reappearance and must-have assertions
  - formal sink / writeback surface: this execution log
  - integration owner/order: tpm integrates before PR
  - context exemption: none
- Validation Command: n/a
- Expected Result: task-bound root-cause debugging proceeds before any further live rebuild attempt.
- Actual Result: task/worktree/log established.
- 遗留事项: inspect cleanup/start wrapper semantics, patch root cause, verify locally, review, PR.
- Blocker / Next Action: collect code and remote service semantics evidence.

## 2026-06-25 08:58:00 CST / tpm
- 完成内容: Implemented next-layer cleanup fix. Live inspection showed `oasis7-testnet-sequencer.service` uses `Restart=on-failure`, `RestartSec=3`, `KillMode=control-group`, and `start-node.sh` backgrounds `oasis7_chain_runtime` then waits/traps cleanup. The post-#627 failure indicates systemd can still re-arm the wrapper after cleanup returned quiet. Patch changes `cleanup_host_processes` to call `systemctl mask --runtime <service>` inside each quiesce iteration before `stop`, `kill --kill-who=all --signal=SIGKILL`, and `reset-failed`; `start_host` now runs `systemctl unmask <service>` before the explicit `reset-failed/start`. The failure cleanup path intentionally leaves the service runtime-masked so no automatic restart source remains after a failed rebuild.
- Test Coverage: updated fake `systemctl` to model masked state. The restart-loop fake now arms on explicit `start`, but cannot spawn a detached stack-root child while masked. The test asserts `unmask` happens before start, cleanup repeatedly masks/stops/kills/resets the service, no fake restart spawn occurs despite the armed loop, no stack-root child survives, and the failed cleanup path leaves the sequencer service masked.
- Validation Command: `bash scripts/p2p-public-testnet-rebuild-validators.test.sh`; `bash -n scripts/p2p-public-testnet-rebuild-validators.sh scripts/p2p-public-testnet-rebuild-validators.test.sh`; `git diff --check`; `./scripts/pm/workflow-lint.sh --task-uid task_22e2decb01b04e7d9cc9f94caecbb308 --phase current`
- Expected Result: focused regression, syntax, diff hygiene, and current task workflow lint pass.
- Actual Result: all four commands passed locally.
- 遗留事项: integrate blockchain_ops_engineer and qa_engineer review findings, then PR.
- Blocker / Next Action: wait for role review results.

## 2026-06-25 09:01:34 CST / tpm
- Review Trigger: pre-PR local role review
- Review Scope: uncommitted diff for `scripts/p2p-public-testnet-rebuild-validators.sh`; `scripts/p2p-public-testnet-rebuild-validators.test.sh`; `doc/p2p/project.md`; `.pm` task truth
- Review Package: current worktree diff; helper review package deferred until after commit because review target is uncommitted.
- Review Roles: blockchain_ops_engineer, qa_engineer, producer_system_designer
- Review Question: confirm cleanup fail-closed runtime masking is the right operator contract for post-cleanup orphan reappearance and that tests lock the behavior without overclaiming live recovery.
- Evidence Available: `bash scripts/p2p-public-testnet-rebuild-validators.test.sh`; `bash -n scripts/p2p-public-testnet-rebuild-validators.sh scripts/p2p-public-testnet-rebuild-validators.test.sh`; `git diff --check`; `./scripts/pm/workflow-lint.sh --task-uid task_22e2decb01b04e7d9cc9f94caecbb308 --phase current`; live post-#627 failure evidence above.
- Expected Return Contract: findings or no_findings; scope/spec compliance verdict; role quality/risk verdict; residual_risk.
- Slice Ledger: n/a, one-shot review slices.
- Formal Sink: `.pm/tasks/task_22e2decb01b04e7d9cc9f94caecbb308.execution.md`
- Review Result: blockchain_ops_engineer returned `no_findings`, scope/spec compliance passed, ops risk acceptable for PR; runtime masking is the right fail-closed control and leaving the service masked after failed cleanup is safer than allowing automatic restarts to mutate validator state. qa_engineer returned `no_findings`, scope/spec compliance passed, test adequacy acceptable; fake systemd tracks masked state, arms restart loop on explicit start, and asserts no spawn while masked. producer_system_designer returned `no_findings`, scope/spec compliance passed, product/system risk acceptable for PR; `doc/p2p/project.md` is scoped to cleanup/fail-closed behavior and does not claim live rebuild recovery, validator health, peer-head recovery, replication recovery, or public testnet health.
- Pre-PR Local Role Review: passed
- Task UID: task_22e2decb01b04e7d9cc9f94caecbb308
- Source Worktree: /Users/scc/ccwork/worktrees/oasis7-p2p-testnet-rebuild-cleanup-post-success-orphan
- Source Branch: task/p2p-testnet-rebuild-cleanup-post-success-orphan
- Source Head: b47b56c05e9ce07bf464210d8569f4da93680875
- Comparison Ref: refs/remotes/origin/main
- Reviewed Changed Paths: scripts/p2p-public-testnet-rebuild-validators.sh; scripts/p2p-public-testnet-rebuild-validators.test.sh; doc/p2p/project.md; .pm/tasks/task_22e2decb01b04e7d9cc9f94caecbb308.execution.md; .pm/tasks/task_22e2decb01b04e7d9cc9f94caecbb308.yaml; .pm/roles/tpm/backlog/committed.yaml
- Review Package: current worktree diff; helper package to be generated by prepare-task-pr after commit.
- Role Selection Basis: node ops cleanup script changed; regression coverage changed; p2p project Trace changed; no player-facing UI/gameplay/WASM/runtime crate change.
- Review Roles: blockchain_ops_engineer, qa_engineer, producer_system_designer
- Review Evidence: blockchain_ops_engineer no_findings; qa_engineer no_findings; producer_system_designer no_findings.
- Review Verdicts: blockchain_ops_engineer scope/spec compliance passed and ops risk acceptable; qa_engineer scope/spec compliance passed and test adequacy acceptable; producer_system_designer scope/spec compliance passed and product/system risk acceptable because project wording makes no live recovery or validator-health claim.
- Review Findings Disposition: no_findings
- Finding Disposition Evidence: n/a
- Verification Matrix: rebuild cleanup script -> focused shell regression passed; shell syntax -> `bash -n` passed; diff hygiene -> `git diff --check` passed; workflow/task truth -> workflow lint passed.
- Visual Evidence: n/a, non-visual ops script change.
- WASM Evidence: n/a, no wasm/runtime artifact change.
- Ops Evidence: live post-#627 failure evidence recorded; post-merge live rebuild still required before claiming validator health.
- LiveOps Evidence: n/a, no external messaging/release claim in this PR.
- Residual Risk: fake ssh/systemd harness cannot prove real ECS cgroup behavior; after merge, rerun live rebuild and sample service mask/active/failed state, `ss/ps/journal/status`, peer-head freshness, replication recent errors, and committed/network heights before observer updates or public testnet health claims.
- Slice Ledger: n/a
- Validation Command: `bash scripts/p2p-public-testnet-rebuild-validators.test.sh`; `bash -n scripts/p2p-public-testnet-rebuild-validators.sh scripts/p2p-public-testnet-rebuild-validators.test.sh`; `git diff --check`; `./scripts/pm/workflow-lint.sh --task-uid task_22e2decb01b04e7d9cc9f94caecbb308 --phase current`
- Expected Result: all review and verification gates pass before PR path.
- Actual Result: all listed gates passed; local role review no_findings.
- 遗留事项: run claim-ready, task closeout, commit, prepare PR.
- Blocker / Next Action: run claim-ready and commit.

## 2026-06-25 09:03:48 CST / tpm
- 完成内容: Ran fresh claim-ready verification for PR readiness.
- Action: verify focused rebuild cleanup regression immediately before committing/PR path.
- Validation Command: `./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command "bash scripts/p2p-public-testnet-rebuild-validators.test.sh"`
- Expected Result: claim-ready reports `allowed_to_claim: true`.
- Actual Result: passed at `2026-06-25T09:03:48+08:00`, `verification_exit_code: 0`, `status: verified`, `allowed_to_claim: true`.
- 遗留事项: commit current task slice and run prepare-task-pr.
- Blocker / Next Action: commit and create PR.

## 2026-06-25 09:12:09 CST / tpm
- 完成内容: Closed task after fresh focused rebuild cleanup regression verification and recorded final task truth.
- Action: task closeout for PR path.
- Validation Command: `./scripts/pm/task-closeout.sh --role tpm --task-uid task_22e2decb01b04e7d9cc9f94caecbb308 --verify-command "bash scripts/p2p-public-testnet-rebuild-validators.test.sh" --no-lint`
- Expected Result: closeout reports final task status `done` and records claim verification status `verified`.
- Actual Result: passed at `2026-06-25T09:12:09+08:00`; final_status `done`; claim_verification_status `verified`; verify_command `bash scripts/p2p-public-testnet-rebuild-validators.test.sh`; pm_lint skipped intentionally because this task uses focused task closeout and repo-wide lint may include unrelated historical debt.
- 遗留事项: create PR, watch required checks/comments, merge, then rerun live validator rebuild before observer updates or public testnet health claims.
- Blocker / Next Action: rerun `./scripts/prepare-task-pr.sh --create`.

## 2026-06-25 09:13:40 CST / tpm
- 完成内容: Created GitHub PR #629 for this task.
- PR URL: https://github.com/eng-cc/oasis7/pull/629
- PR Purpose Decision: normal_pr_ci_watch
- Rationale: ordinary implementation/documentation fix for validator rebuild cleanup. This PR is not a manual packaging/release CI hold.
- Required Follow-up: watch required checks, mergeability, PR comments, and review threads; treat `REVIEW_REQUIRED`/approval-only `BLOCKED` as informational under repo policy, but do not merge with failing checks, requested changes, unresolved blocking comments/threads, conflicts, or a GitHub merge API refusal.
- Post-Merge Gate: rerun live validator rebuild and sample service mask/active/failed state, ports/processes, peer-head freshness, replication recent errors, and committed/network heights before observer updates or public testnet health claims.

## 2026-06-25 09:24:30 CST / tpm
- 完成内容: Addressed GitHub PR #629 review thread `PRRT_kwDORHhWec6MFOJu`.
- Review Classification: valid regression risk. `systemctl mask --runtime` is the fail-closed safety prerequisite for this root-cause fix; silently continuing with `check=False` could allow cleanup to proceed without disabling the service restart source.
- Action: changed cleanup quiesce logic so runtime mask failure prints `cleanup failed: systemctl runtime mask failed ...` and exits the cleanup command immediately; added fake systemd regression coverage that injects a sequencer runtime-mask failure and expects the rebuild to fail before continuing.
- Validation Command: `bash -n scripts/p2p-public-testnet-rebuild-validators.sh scripts/p2p-public-testnet-rebuild-validators.test.sh`; `git diff --check`; `bash scripts/p2p-public-testnet-rebuild-validators.test.sh`
- Expected Result: syntax and diff hygiene pass; focused rebuild cleanup regression passes including the new runtime-mask failure assertion.
- Actual Result: all three checks passed locally.
- 遗留事项: push review fix, resolve the GitHub thread, continue required checks/comment/mergeability watch.
- Blocker / Next Action: commit and push review fix.
