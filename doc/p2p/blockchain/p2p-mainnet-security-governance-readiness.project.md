# 主网安全、治理与创世就绪度（项目追踪）

- 对应需求文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`
- 对应设计文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.design.md`

## 稳定追踪边界

本页仅追踪专业文档 authority consolidation 及其仍未关闭的技术/证据前置条件；当前任务状态与执行证据由 GitHub task issue evidence 承载。

## 任务拆解

- [x] p2p-mainnet-security-governance-authority-consolidation (PRD-P2P-MAINNET-001/002/003/004) [test_tier_required]: 吸收历史 crypto/readiness/custody/governance/genesis 合同并保留负向 gate。 Trace: #2672 (task_b23df712ec4d4bb8b314c84a50278873)
- [x] p2p-mainnet-signed-transaction-authorization-consolidation (PRD-P2P-MAINNET-005) [test_tier_required]: 吸收 HTTP signature/account binding、canonical proof、shared-layer re-verification、nonce-replay boundary，以及 native/wasm 同协议本地产签、`__OASIS7_VIEWER_AUTH_ENV` bootstrap 注入与缺失/partial bootstrap 的 fail-closed submission；保留 non-custody/not-mainnet-grade 约束。 Trace: #2682 (task_172abebb99354d4fad395aa05a581193)

未关闭的专业条件：

- 历史专题中“规格已定义”与“真实操作/QA 未完成”的区别必须持续保留。
- production custody：受控 backend、rotation、revocation 与审计链需由 runtime/blockchain ops 后续以当前证据验证。
- genesis：真实 recipient/controller binding、ceremony 与 QA `pass` 仍是 `not_mint_ready` 的阻断项。
- governance：shared-network probation、扩展 rotation/revocation/failover 覆盖仍需对应专业证据。
- mainnet：formal tier、frozen genesis、no-reset commitment 和全部跨域 gate 必须共同满足；本页不授予升级结论。

## 验证入口

- 文档治理：`./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh`。
- signed transfer client contract：历史 `test_tier_required` 覆盖 request builder/serialization/wasm compile，`test_tier_full` 覆盖 Web-first signed submit 与 bootstrap 缺失的本地阻断；证据见 `doc/testing/evidence/mainchain-token-signed-transfer-web-validation-2026-03-23.md`。
- registry 合同：`./scripts/governance-registry-drill.sh`（按其安全输入要求执行）。
- network tier：`./scripts/network-tier-manifest-smoke.sh`。

历史专题的任务拆解与完成过程仅从 Git history 与 GitHub task evidence 追溯，不再维持为 active project authority。

## 依赖

- 本专题“资产与密码学基线”中的 signed transaction contract
- `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`
- `testing-manual.md`

## 状态

- 当前阶段：active authority；系统级状态仍为 `not_mainnet_grade`，创世仍为 `not_mint_ready`。
