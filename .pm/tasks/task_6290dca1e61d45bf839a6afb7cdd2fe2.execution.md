# task_6290dca1e61d45bf839a6afb7cdd2fe2 Execution Log

- task_uid: task_6290dca1e61d45bf839a6afb7cdd2fe2
- title: shrink node consensus signatures hotspot
- owner_role: runtime_engineer
- worktree_hint: null

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-22 13:08:00 CST / runtime_engineer
- 完成内容: 将 `crates/oasis7_node/src/tests_consensus_signatures.rs` 中的 replica maintenance 配置/轮询测试抽到新的 `crates/oasis7_node/src/tests_runtime_replica_maintenance.rs`，并在父模块新增 `#[path = "tests_runtime_replica_maintenance.rs"] mod runtime_replica_maintenance_tests;` 维持原测试入口；主文件从 1172 行降到 1079 行，新文件为 96 行。
- 完成内容: 验证通过 `env -u RUSTC_WRAPPER cargo check -p oasis7_node --lib`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node config_rejects_non_positive_replica_maintenance_poll_interval -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replica_maintenance_poll_executes_local_target_tasks -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replica_maintenance_poll_skips_without_dht -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node pos_engine_signature_enforced_accepts_signed_proposal_and_attestation -- --nocapture`、`./scripts/check-rust-file-size.sh`、`./scripts/doc-governance-check.sh`、`git diff --check`。
- 遗留事项: near-limit 清单的下一优先热点已经转移到 `crates/oasis7_node/src/tests_network_gap_sync.rs` 1141 行；`tests_consensus_signatures.rs` 仍是 1079 行，后续功能继续堆积前应优先再做预防性拆分。
