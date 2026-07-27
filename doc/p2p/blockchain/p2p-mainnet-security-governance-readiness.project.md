# 主网安全、治理与创世就绪度（项目追踪）

- 对应需求文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`
- 对应设计文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.design.md`

## 稳定追踪边界

本页仅追踪专业文档 authority consolidation 及其仍未关闭的技术/证据前置条件；当前任务状态与执行证据由 GitHub task issue evidence 承载。

## 任务拆解

- [x] p2p-mainnet-security-governance-authority-consolidation (PRD-P2P-MAINNET-001/002/003/004) [test_tier_required]: 吸收历史 crypto/readiness/custody/governance/genesis 合同并保留负向 gate。 Trace: #2672 (task_b23df712ec4d4bb8b314c84a50278873)
- [x] 保留历史专题中“规格已定义”与“真实操作/QA 未完成”的区别。
- [ ] production custody：受控 backend、rotation、revocation 与审计链需由 runtime/blockchain ops 后续以当前证据验证。
- [ ] genesis：真实 recipient/controller binding、ceremony 与 QA `pass` 仍是 `not_mint_ready` 的阻断项。
- [ ] governance：shared-network probation、扩展 rotation/revocation/failover 覆盖仍需对应专业证据。
- [ ] mainnet：formal tier、frozen genesis、no-reset commitment 和全部跨域 gate 必须共同满足；本页不授予升级结论。

## 验证入口

- 文档治理：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`。
- registry 合同：`./scripts/governance-registry-drill.sh`（按其安全输入要求执行）。
- network tier：`./scripts/network-tier-manifest-smoke.sh`。

历史专题的任务拆解与完成过程仅从 Git history 与 GitHub task evidence 追溯，不再维持为 active project authority。

## 依赖

- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`
- `testing-manual.md`

## 状态

- 当前阶段：active authority；系统级状态仍为 `not_mainnet_grade`，创世仍为 `not_mint_ready`。
