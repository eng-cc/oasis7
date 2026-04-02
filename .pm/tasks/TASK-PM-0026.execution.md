# TASK-PM-0026 Execution Log

- task_id: TASK-PM-0026
- title: Implement P2PARCH-2 hole-punch and relay reservation substrate
- owner_role: runtime_engineer
- worktree_hint: oasis7-p2p-p2parch-2-hole-punch-relay-reservation

## 2026-04-02 16:19:56 CST / runtime_engineer
- 完成内容: 为 `P2PARCH-2` 补齐 hole-punch / relay reservation substrate。`PeerRecord` 新增 `hole_punch_addrs`，peer record materialization 会把 direct listener 与 `/p2p-circuit` relayed listener 分离到 `direct_addrs / relay_addrs`；transport substrate 现在显式区分 `Direct / HolePunched / RelayReserved` 与 `Quic / TcpNoiseYamux / RelayTunnel`，并按 `direct QUIC -> direct TCP -> hole-punched QUIC/TCP -> relay-reserved` 排序与 failover。swarm 接入 relay client transport 与 DCUtR behaviour，runtime 会记录 relay reservation / relay circuit / hole-punch 成败事件，并在 reservation accepted 后刷新 peer record/provider 广告。补齐 `oasis7_net` / `oasis7_proto` / `oasis7_chain_runtime` 相关测试与专题 `project.md` 追踪文档。
- 完成内容: 验证已通过 `env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib`、`env -u RUSTC_WRAPPER cargo test -p oasis7_node --lib`、`git diff --check`；此前单次 `oasis7_node` 全包测试出现过 `runtime_network_replication_fetch_handlers_serve_commit_and_blob` 波动，复跑单测与整包后均稳定通过，当前未复现。
- 遗留事项: 补做 manual diff review、PM close、commit 与标准化 landing；AutoNAT -> hole punch -> relay reservation 的 lifecycle 自动化与 mixed-topology 套件仍属后续 `P2PARCH-3/6` 范围。

## 2026-04-02 16:22:52 CST / runtime_engineer
- 完成内容: 受当前会话 delegation 约束限制，未启独立 subagent；已对 `crates/oasis7_net` / `crates/oasis7_proto` / `crates/oasis7/src/bin/oasis7_chain_runtime.rs` / 专题 `project.md` diff 做人工 review 代替。未发现会破坏 direct path、peer-record 签名校验或 node replication 的阻断性回归；relay transport 与 DCUtR 当前仅补 substrate / event surface，没有伪装成已实现的 reachability lifecycle 自动化。
- 完成内容: 文档与流程门禁已通过 `./scripts/doc-governance-check.sh`、`./scripts/pm/lint.sh`。
- 遗留事项: 执行 PM close、将任务迁移到 `done`、提交 commit 并通过 `./scripts/land-task-worktree.sh` 合入本地 `main`。
