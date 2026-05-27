# oasis7 Runtime：可兑现节点资产与电力兑换闭环（三期真实签名与治理闭环，项目管理文档）（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.design.md`
- 对应需求文档: `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.prd.md`

审计轮次: 5
## 审计备注
- 项目主入口文档：`doc/p2p/node/node-redeemable-power-asset.project.md`。
- 本文件仅维护“三期真实签名与治理闭环”增量任务。
- 通用任务与状态口径以主项目文档为准。

## 任务拆解（含 PRD-ID 映射）
- [x] SGC-0 (PRD-P2P-MIG-097)：完成三期设计文档。
- [x] SGC-1 (PRD-P2P-MIG-097)：完成三期项目管理文档拆解。
- [x] SGC-2 (PRD-P2P-MIG-097)：落地 `mintsig:v2`（ed25519）签名/验签与治理策略结构。
- [x] SGC-3 (PRD-P2P-MIG-097)：实现签名版兑换动作与治理门禁（策略要求时拒绝无签名兑换）。
- [x] SGC-4 (PRD-P2P-MIG-097)：接线 `oasis7_viewer_live` reward runtime（真实私钥结算 + 签名兑换）。
- [x] SGC-5 (PRD-P2P-MIG-097)：补齐 `test_tier_required` 回归、回写文档状态与 devlog 收口。

## 依赖
- `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.prd.md`
- `crates/oasis7/src/runtime/reward_asset.rs`
- `crates/oasis7/src/runtime/world/resources.rs`
- `crates/oasis7/src/runtime/world/event_processing.rs`
- `crates/oasis7/src/runtime/events.rs`
- `crates/oasis7/src/runtime/tests/reward_asset.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：SGC-0 ~ SGC-5 全部完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
