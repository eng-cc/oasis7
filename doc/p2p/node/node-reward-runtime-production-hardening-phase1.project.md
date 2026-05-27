# oasis7 Runtime：节点奖励运行时生产化加固（Phase 1，项目管理文档）（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-reward-runtime-production-hardening-phase1.design.md`
- 对应需求文档: `doc/p2p/node/node-reward-runtime-production-hardening-phase1.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] PRH1-1 (PRD-P2P-MIG-100)：完成设计文档与项目管理文档。
- [x] PRH1-2 (PRD-P2P-MIG-100)：实现 `NodePointsLedger/Collector` 快照导出与恢复，并接入 reward runtime 状态文件。
- [x] PRH1-3 (PRD-P2P-MIG-100)：实现兑换签名授权策略 `require_redeem_signer_match_node_id` 并接入默认生产配置。
- [x] PRH1-4 (PRD-P2P-MIG-100)：移除 reward runtime 占位身份绑定，改为显式绑定与错误收口。
- [x] PRH1-5 (PRD-P2P-MIG-100)：补齐/更新测试，执行 `test_tier_required` 回归，回写文档状态与 devlog。

## 依赖
- doc/p2p/node/node-reward-runtime-production-hardening-phase1.prd.md
- `crates/oasis7/src/runtime/node_points.rs`
- `crates/oasis7/src/runtime/node_points_runtime.rs`
- `crates/oasis7/src/runtime/reward_asset.rs`
- `crates/oasis7/src/runtime/world/event_processing.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：PRH1-1 ~ PRH1-5 已完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
