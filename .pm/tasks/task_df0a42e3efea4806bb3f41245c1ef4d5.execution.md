# task_df0a42e3efea4806bb3f41245c1ef4d5 Execution Log

- task_uid: task_df0a42e3efea4806bb3f41245c1ef4d5
- title: Reduce fetch-commit retry traffic waste
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-world-runtime-fetch-commit-retry-backoff

## 2026-04-16 14:42:28 CST / runtime_engineer
- 完成内容: 在 `crates/oasis7_node/src/libp2p_replication_network.rs` 为 `fetch-commit` 请求增加协议级短时 peer cooldown，把最近刚返回 `ErrNotFound`、`Timeout`、连接缺口或已有 missing-handler/unsupported 签名的 peer 暂时排除出下一轮候选，减少真实 triad gap-sync 对同一无效目标的重复 libp2p 请求；同步补 `doc/world-runtime/prd.md` / `doc/world-runtime/project.md` 的 `PRD-WORLD_RUNTIME-029` 映射，并在 `crates/oasis7_node/src/libp2p_replication_network/tests.rs` 新增协议范围与立即重试抑制回归。
- 完成内容: 已验证 `env -u RUSTC_WRAPPER cargo test -p oasis7_node libp2p_replication_network -- --nocapture`、`./scripts/doc-governance-check.sh`、`git diff --check` 全部通过。
- 遗留事项: 需要在当前 stacked branch 完成 commit / PR 后，按真实 triad 部署当前版本，再对比最近窗口中的 `fetch-commit` 噪音与 libp2p replication bytes/message delta 是否继续下降。

## 2026-04-16 22:52:00 CST / runtime_engineer
- 完成内容: 按 PR #100 review comment 收口 `PRD-WORLD_RUNTIME-029` 文档口径，明确短时退避只覆盖 missing-handler/unsupported-protocol 签名的 `ErrUnsupported`、`ErrNotFound`、`Timeout` 与连接缺口，不把泛化业务态 `ErrUnsupported` 误写成触发条件。
- 完成内容: 将 `libp2p_replication_network` 与 `oasis7_chain_runtime` debug/status 中的 `unsupported_protocol_*` 命名统一改为更准确的 `protocol_retry_cooldown_*`，避免未来把该短时退避误解成“只处理 unsupported protocol”。
- 遗留事项: 待本轮定向验证通过后提交、推送并回看 PR #100 的 review thread / mergeability 状态。
