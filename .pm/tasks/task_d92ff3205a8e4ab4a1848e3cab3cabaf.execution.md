# task_d92ff3205a8e4ab4a1848e3cab3cabaf Execution Log

- task_uid: task_d92ff3205a8e4ab4a1848e3cab3cabaf
- title: shrink node consensus signatures hotspot follow-up
- owner_role: runtime_engineer
- worktree_hint: null

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-22 13:39:00 CST / runtime_engineer
- 完成内容: 将 `crates/oasis7_node/src/tests_consensus_signatures.rs` 中的 POS 引擎基础行为 / overflow guard / runtime lifecycle 测试抽到新的 `crates/oasis7_node/src/tests_pos_engine_guardrails.rs`，并在父模块新增 `#[path = "tests_pos_engine_guardrails.rs"] mod pos_engine_guardrails_tests;` 维持原测试入口；主文件从 1080 行降到 778 行，新文件为 305 行。
- 完成内容: 验证通过 `env -u RUSTC_WRAPPER cargo check -p oasis7_node --lib`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node pos_engine_apply_decision_rejects_height_overflow_without_state_mutation -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_start_and_stop_updates_snapshot -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node pos_engine_signature_enforced_accepts_signed_proposal_and_attestation -- --nocapture`、`./scripts/check-rust-file-size.sh`、`./scripts/doc-governance-check.sh`、`git diff --check`。
- 遗留事项: `tests_consensus_signatures.rs` 与 `tests_network_gap_sync.rs` 已经都回到安全区，下一轮应重新扫描 near-limit 清单，避免继续围着已恢复余量的入口做无效切分。
