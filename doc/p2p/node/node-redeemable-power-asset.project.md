# oasis7 Runtime：可兑现节点资产与电力兑换闭环（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-redeemable-power-asset.design.md`
- 对应需求文档: `doc/p2p/node/node-redeemable-power-asset.prd.md`

审计轮次: 5
## ROUND-002 主从口径
- 项目主入口文档：`doc/p2p/node/node-redeemable-power-asset.project.md`。
- 增量项目文档：`doc/p2p/node/node-redeemable-power-asset-audit-hardening.project.md`、`doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.project.md`。
- 增量项目文档仅维护增量任务，通用任务与状态口径以主项目文档为准。

## 任务拆解（含 PRD-ID 映射）
- [x] RPA-0 (PRD-P2P-MIG-098)：完成设计文档与项目管理文档。
- [x] RPA-1 (PRD-P2P-MIG-098)：实现 `PowerCredit` 资产账本与配置（含快照持久化）。
- [x] RPA-2 (PRD-P2P-MIG-098)：将 `NodePoints` epoch 结算结果接入链状态铸造记录（`NodeRewardMintRecord`）。
- [x] RPA-3 (PRD-P2P-MIG-098)：实现 `RedeemPower` 动作闭环（余额扣减、Agent 电力增加、事件产出）。
- [x] RPA-4 (PRD-P2P-MIG-098)：实现守恒与风控（储备池、每 epoch 额度、最小兑换单位、nonce 防重放）。
- [x] RPA-5 (PRD-P2P-MIG-098)：接线运行时主链路（`oasis7_viewer_live`/runtime 开关与配置）。
- [x] RPA-6 (PRD-P2P-MIG-098)：实现最小需求侧支付入口（系统订单池预算分配）并接入结算。
- [x] RPA-7 (PRD-P2P-MIG-098)：实现身份签名治理最小收口（`node_id <-> public_key` 校验，拒绝未绑定提交）。
- [x] RPA-8 (PRD-P2P-MIG-098)：增强 DistFS 证明语义字段并补齐 `test_tier_required`/`test_tier_full` 回归。
- [x] RPA-9 (PRD-P2P-MIG-098)：文档状态回写、devlog 收口、发布说明整理。

## 依赖
- `doc/p2p/node/node-redeemable-power-asset.prd.md`
- `crates/oasis7/src/runtime/node_points.rs`
- `crates/oasis7/src/runtime/node_points_runtime.rs`
- `crates/oasis7/src/runtime/state.rs`
- `crates/oasis7/src/runtime/events.rs`
- `crates/oasis7_proto/src/distributed.rs`
- `crates/oasis7_consensus/src/pos.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7_distfs/src/lib.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：RPA-0 ~ RPA-9 全部完成。
- 下一步：等待验收与后续迭代需求。
- 最近更新：2026-02-16。
