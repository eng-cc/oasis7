# P2P Mobile Light Client 权威状态架构（项目管理文档）

- 对应设计文档: `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.design.md`
- 对应需求文档: `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-P2P-MLC-001 (PRD-P2P-MLC-001/002/003/004) [test_tier_required]: 输出专题 PRD、项目管理文档，并回写模块主 PRD/索引链路。
- [x] TASK-P2P-MLC-002 (PRD-P2P-MLC-001) [test_tier_required]: 实现移动端 intent-only 协议字段（`tick/seq/sig`）与网关去重/幂等 ACK。
- [x] TASK-P2P-MLC-003 (PRD-P2P-MLC-002) [test_tier_required]: 实现权威批次提交（`state_root/data_root`）和客户端 `pending/confirmed/final` 状态机。
- [x] TASK-P2P-MLC-004 (PRD-P2P-MLC-002/003) [test_tier_full]: 实现 challenge/resolve/slash 链路与 watcher 复算入口。
- [x] TASK-P2P-MLC-005 (PRD-P2P-MLC-004) [test_tier_required]: 实现链重组回滚、断线重连快照追平、会话吊销换钥流程。
- [x] TASK-P2P-MLC-006 (PRD-P2P-MLC-001/002/003/004) [test_tier_full]: 执行 required/full 联合回归并沉淀发布门禁证据。

### TASK-P2P-MLC-003 执行拆解（PRD-P2P-MLC-002）
- [x] TASK-P2P-MLC-003-A [test_tier_required]: 在权威执行提交链路补齐批次承诺结构，确保 `batch_id/state_root/data_root` 同步产出与持久化。
- [x] TASK-P2P-MLC-003-B [test_tier_required]: 在客户端消费链路实现 `pending -> confirmed -> final` 最终性状态机与单调迁移约束。
- [x] TASK-P2P-MLC-003-C [test_tier_required]: 在结算/排行入口增加最终性闸门，非 `final` 数据禁止进入资产结算与排行统计。
- [x] TASK-P2P-MLC-003-D [test_tier_required]: 补齐 `state_root/data_root` 与 finality 状态机定向测试，并执行 `testing-manual.md` 对应 required 套件。

### TASK-P2P-MLC-004 执行拆解（PRD-P2P-MLC-002/003）
- [x] TASK-P2P-MLC-004-A [test_tier_full]: 在 `runtime_live` 增加 challenge 提交结构（`challenge_id/batch_id/recomputed_state_root/recomputed_data_root`）与 watcher 复算入口。
- [x] TASK-P2P-MLC-004-B [test_tier_full]: 实现 `challenged -> resolved` 仲裁状态机，根不一致分支阻断 final。
- [x] TASK-P2P-MLC-004-C [test_tier_full]: 实现 slash 记录与批次联动（错误根触发 slash、正确根不罚没），并保证重复 challenge/resolve 幂等拒绝。
- [x] TASK-P2P-MLC-004-D [test_tier_full]: 补齐 challenge/resolve/slash 定向测试并执行 `testing-manual.md` 对应 full 套件。

### TASK-P2P-MLC-005 执行拆解（PRD-P2P-MLC-004）
- [x] TASK-P2P-MLC-005-A [test_tier_required]: 在 `runtime_live` 引入稳定检查点与回滚入口，支持按稳定批次执行重组回滚并清理分叉批次最终性。
- [x] TASK-P2P-MLC-005-B [test_tier_required]: 增加断线重连追平元数据（`snapshot_hash/log_cursor/stable_batch_id/reorg_epoch`）与快照重拉分支，确保游标缺口可恢复。
- [x] TASK-P2P-MLC-005-C [test_tier_required]: 实现会话吊销/换钥命令，落地旧 key 拒绝、新 key 绑定与 epoch 单调约束。
- [x] TASK-P2P-MLC-005-D [test_tier_required]: 补齐重组回滚、重连追平、会话吊销换钥的定向测试并执行 `testing-manual.md` 对应 required 套件。

### TASK-P2P-MLC-006 执行拆解（PRD-P2P-MLC-001/002/003/004）
- [x] TASK-P2P-MLC-006-A [test_tier_required]: 执行 MLC 链路 required 回归矩阵（intent/finality/recovery 关键用例）并归档命令与结果。
- [x] TASK-P2P-MLC-006-B [test_tier_full]: 执行 MLC 链路 full 回归矩阵（challenge/slash/recovery 组合路径）并归档命令与结果。
- [x] TASK-P2P-MLC-006-C [test_tier_full]: 汇总联合回归证据，形成 PRD-ID -> Test -> 结果映射与失败项说明（含基线缺陷隔离）。
- [x] TASK-P2P-MLC-006-D [test_tier_full]: 更新发布门禁结论与风险说明，确认是否满足进入下一阶段的回归准入条件。

## 依赖
- `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md`
- `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.project.md`
- `doc/p2p/prd.md`
- `doc/p2p/network/readme-p1-network-production-hardening.prd.md`
- `doc/p2p/distributed/distributed-runtime.prd.md`
- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`
- `testing-manual.md`

## 状态
- 更新日期: 2026-03-07
- 当前状态: active
- 下一任务: 无（TASK-P2P-MLC-001~006 已完成）
- TASK-P2P-MLC-003 收口（2026-03-07）: A/B/C/D 已全部完成（批次承诺、最终性状态机、结算闸门、required 定向回归）。
- TASK-P2P-MLC-004 收口（2026-03-07）: A/B/C/D 已全部完成（challenge 提交入口、resolve 状态机、slash 联动、full 定向回归）。
- TASK-P2P-MLC-005 收口（2026-03-07）: A/B/C/D 已全部完成（稳定点回滚、重连追平元数据、会话吊销换钥、required 定向回归）。
- TASK-P2P-MLC-006 收口（2026-03-07）: A/B/C/D 已全部完成（required/full 联合回归、证据映射、发布门禁结论）。
- 本轮完成:
  - 在 `oasis7_proto::viewer::AgentChatRequest` 增加 `intent_tick/intent_seq` 字段，并在 `AgentChatAck` 增加 `intent_tick/intent_seq/idempotent_replay`。
  - `runtime_live` 增加 `intent_seq` 幂等重放语义：同 `(player_id, agent_id, intent_seq)` 重试返回同 ACK，变更载荷触发冲突拒绝。
  - `viewer/auth` 的 agent_chat 签名载荷纳入 `intent_tick/intent_seq`，并校验 `intent_seq > 0`。
  - `runtime_live` 新增权威批次提交记录：每 step 产出并保留 `batch_id/state_root/data_root`，并通过 `authoritative_batch` 响应下发客户端。
  - 落地 `pending -> confirmed -> final` 单调最终性状态机；根缺失/格式异常/`data_root` 校验不一致时批次保持 `pending`。
  - 落地结算/排行最终性闸门：仅 `final` 批次可标记 `settlement_ready/ranking_ready=true`。
  - 协议新增 `authoritative_challenge` 请求与 `authoritative_challenge_ack/error` 响应，支持 watcher 提交复算根与仲裁 resolve。
  - `runtime_live` 落地 challenge/resolve/slash：`challenge_open` 阻断 final，`ResolvedFraudSlashed` 阻断最终化并记录 slash，`ResolvedNoFraud` 允许继续最终化。
  - 批次可见字段补齐：`challenge_open/slashed/active_challenge_id`，并在快照请求时回放 challenge 历史 ACK。
  - 协议新增 `authoritative_recovery` 请求与 `authoritative_recovery_ack/error` 响应，覆盖 rollback/reconnect_sync/revoke_session/rotate_session 四类恢复操作。
  - `runtime_live` 新增稳定检查点与 `reorg_epoch`，支持按稳定批次执行重组回滚并在回滚后补发快照与批次追平数据。
  - `RequestSnapshot` 增加恢复追平元数据（`snapshot_hash/log_cursor/stable_batch_id/reorg_epoch`），用于重连时判断是否需要强制重拉快照。
  - 新增会话策略（active/revoked/session_epoch），实现会话吊销换钥并联动 `agent_chat/prompt_control` 鉴权拦截（旧 key 全拒绝）。
  - 定向回归通过:
    - `env -u RUSTC_WRAPPER cargo check -p oasis7_proto -p oasis7 -p oasis7_viewer`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_response_round_trip_authoritative_batch`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_authoritative_batch_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_authoritative_challenge_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_response_round_trip_authoritative_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full runtime_authoritative_challenge_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_authoritative_recovery_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_response_round_trip_authoritative_recovery_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_authoritative_recovery_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime_authoritative_recovery_`
  - `TASK-P2P-MLC-006` 联合回归证据（required/full）:
    - `env -u RUSTC_WRAPPER cargo check -p oasis7_proto -p oasis7 -p oasis7_viewer`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_authoritative_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_proto viewer_response_round_trip_authoritative_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime_authoritative_batch_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime_authoritative_recovery_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full runtime_authoritative_challenge_`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full runtime_authoritative_recovery_`
- 风险提示:
  - challenge 规则与实时体验存在冲突，需要联动客户端最终性文案。
  - 快照/日志可用性会直接影响重连追平成功率。
  - 全仓 pre-commit required 套件存在已知基线失败（`bootstrap_power` / `agent_default_modules` 等约 10 项），本任务仅对 MLC 相关矩阵执行并通过；全仓门禁修复需另立专项追踪。
- 说明: 本文档只维护执行计划；过程记录写入 `doc/devlog/README.md`。
