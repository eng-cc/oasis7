# oasis7 Runtime：可兑现节点资产与电力兑换闭环（二期审计与签名加固，项目管理文档）（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-redeemable-power-asset-audit-hardening.design.md`
- 对应需求文档: `doc/p2p/node/node-redeemable-power-asset-audit-hardening.prd.md`

审计轮次: 5
## 审计备注
- 项目主入口文档：`doc/p2p/node/node-redeemable-power-asset.project.md`。
- 本文件仅维护“二期审计与签名加固”增量任务。
- 通用任务与状态口径以主项目文档为准。

## 任务拆解（含 PRD-ID 映射）
- [x] AHA-0 (PRD-P2P-MIG-096)：完成设计文档。
- [x] AHA-1 (PRD-P2P-MIG-096)：完成项目管理文档拆解。
- [x] AHA-2 (PRD-P2P-MIG-096)：实现结算签名语义升级（`mintsig:v1`）与签名校验函数。
- [x] AHA-3 (PRD-P2P-MIG-096)：实现资产不变量审计报告 API，并覆盖 `test_tier_required` 单测。
- [x] AHA-4 (PRD-P2P-MIG-096)：`oasis7_viewer_live` reward runtime 接入审计摘要输出。
- [x] AHA-5 (PRD-P2P-MIG-096)：文档状态回写、devlog 收口、发布说明整理。

## 依赖
- `doc/p2p/node/node-redeemable-power-asset-audit-hardening.prd.md`
- `crates/oasis7/src/runtime/reward_asset.rs`
- `crates/oasis7/src/runtime/world/resources.rs`
- `crates/oasis7/src/runtime/tests/reward_asset.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：AHA-0 ~ AHA-5 全部完成。
- 下一步：等待验收与后续迭代需求。
- 最近更新：2026-02-16。
