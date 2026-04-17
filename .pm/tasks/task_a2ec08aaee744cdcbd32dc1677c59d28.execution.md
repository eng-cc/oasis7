# task_a2ec08aaee744cdcbd32dc1677c59d28 Execution Log

- task_uid: task_a2ec08aaee744cdcbd32dc1677c59d28
- title: burn down structural slicing in oasis7_node lib
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-engineering-rust-structural-slicing-oasis7-node-lib

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-04-17 21:27:59 CST / runtime_engineer
- 完成内容: 将 `crates/oasis7_node/src/lib.rs` 从 `include!/impl_part` 切换为正常 `mod` 入口，迁移为 `node_engine_core.rs`、`node_engine_network.rs`、`node_engine_storage_challenge.rs`，退役 `lib_impl_part1.rs` / `lib_impl_part2.rs` / `lib_impl_storage_challenge.rs` 三个旧分片文件名。
- 完成内容: 针对真实子模块后的可见性收敛，将根模块、`pos_engine_gossip`、`replication_probe_gate`、`replication_state_reconcile` 与测试所需的 `PosNodeEngine` helper 明确收敛到 `pub(super)`，避免把原本默认同模块可见的 helper 扩散成更大范围公开接口。
- 完成内容: 回写 `doc/.governance/rust-structural-slicing-baseline.tsv`，退役 `oasis7_node/lib.rs` 相关 4 条 frozen structural slicing 记录；验证 `env -u RUSTC_WRAPPER cargo check -p oasis7_node`、`./scripts/check-rust-file-size.sh`、`git diff --check` 与 4 条 node engine 精确用例通过。
- 完成内容: 复跑 `cargo test -p oasis7_node --lib -- --nocapture`，确认当前仍存在 3 个现存失败签名：`tests::non_sequencer_followers::runtime_network_replication_respects_topic_isolation`、`tests_hardening::runtime_fetch_handlers_reject_unsigned_fetch_request_in_signed_mode`、`tests::runtime_gossip_replication_persists_guard_across_restart`；本轮未修改其行为。
- 遗留事项: 下一批结构债建议继续处理 `crates/oasis7_net/src/libp2p_net.rs` 或 `crates/oasis7_client_launcher/src/self_guided.rs`；若要把 `oasis7_node --lib` 拉到全绿，需要单独开 task 处理上述 3 个失败签名。
