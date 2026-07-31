# 零信任多节点治理与签名加固设计（2026-02-26）

- 对应需求文档: `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

## 1. 设计定位
定义 runtime 在不可信多节点环境下的工件验签、治理最终性绑定、收据签名升级与执行错误可观测性设计。

## 2. 设计结构
- 工件真实性链：统一 `artifact_identity` 校验、签名方案与受信任签名人集合。
- 治理原子 apply：把 proposal 校验、多签验证、模块变更应用与事件落档收敛到单一事务顺序。
- 收据签名层：兼容 HMAC 历史方案，同时引入节点签名/阈值签名与共识锚定字段。
- 错误观测层：将执行 trap 明确映射为 `OutOfFuel` / `Interrupted`。

## 3. 关键接口 / 入口
- `apply_proposal(proposal_id, finality_certificate)`
- `GovernanceFinalityCertificate` / `ReceiptSignature` / `SignatureAlgorithm`
- `validate_module_manifest` / `load_module_store_from_dir`

## 4. 约束与边界
- 不破坏现有 WASM ABI 与事件溯源框架。
- 任意治理落地必须绑定最终性证明与多签门限。
- 收据锚定字段必须可追溯到 `consensus_height` 与 `receipts_root`。

## 5. 设计演进计划
- 先完成设计补齐与互链回写。
- 再按项目文档任务拆解推进实现与验证。
