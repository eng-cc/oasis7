# TASK-PM-0025 Execution Log

- task_id: TASK-PM-0025
- title: Implement P2PARCH-2 QUIC TCP fallback
- owner_role: runtime_engineer
- worktree_hint: oasis7-p2p-p2parch-2-quic-tcp-fallback

## 2026-04-02 14:56:04 CST / runtime_engineer
- 完成内容: 在 `crates/oasis7_net` 打开 `libp2p` 的 `quic` feature，并把 `build_swarm` 改成 `QUIC + TCP/Noise/Yamux` 的 `OrTransport` 组合；同步扩展 `transport_paths` substrate，把 direct/relay 之外的传输 flavor 细化为 `quic` 与 `tcp+noise+yamux`，固定当前优先级为 `direct QUIC -> direct TCP -> relay`。
- 完成内容: 调整 discovery 的优先级比较逻辑，使已建立 direct TCP 连接在发现更优 direct QUIC path 时可升级；同时把 active transport 日志补成 `kind + flavor`，便于后续 mixed-topology 验证区分主路径与 fallback。
- 完成内容: 新增并通过 `oasis7_net` libp2p 单测，覆盖 peer record path 排序、QUIC 失败后的 TCP fallback、endpoint 对 QUIC/relay 语义的推断，以及全量 `env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib` 回归。
- 遗留事项: `P2PARCH-2` 仍未补 hole-punched path、relay reservation 与更完整 mixed-topology failover 套件；本任务只收敛 QUIC/TCP fallback 切片。

## 2026-04-02 14:59:37 CST / runtime_engineer
- 完成内容: 已执行人工 diff review，确认 QUIC/TCP fallback 仍只发生在已通过 peer record 发现并入库的 transport path 集合内，没有引入绕过 world/network/signature 校验的旁路；同时完成 `git diff --check`、`./scripts/doc-governance-check.sh`、`./scripts/pm/lint.sh`、`env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib` 与 `env -u RUSTC_WRAPPER cargo test -p oasis7_node --lib` 回归。
- 遗留事项: 仓库工作流默认要求 commit 前做 subagent review，但当前会话工具策略不允许在未获用户明确授权时启动 `spawn_agent`；本任务以人工 diff review 替代，并在任务日志中显式记录该例外。
