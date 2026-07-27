# 主网安全、治理与创世就绪度设计

- 对应需求文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.prd.md`
- 对应项目管理文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.project.md`

## 设计原则

1. 将历史专题按 authority 合并，而不是将历史完成状态升级为当前状态。
2. 保持 `registry-first`：world-state registry 是治理 membership、stake、finality signer 与 controller policy 的运行真值。
3. 以 fail-closed 处理 signer/custody、genesis binding、ceremony 和 QA 证据缺失。
4. 将正式网络层级、运行步骤和公开沟通分别下钻到 formal-network-tier、runbook 和相应专业权威。

## 合并结构

| 历史主题 | 吸收后的稳定章节 | 不应丢失的限制 |
| --- | --- | --- |
| crypto baseline | 资产与密码学基线 | 局部原语/授权完成不等于系统级安全。 |
| MAINNET readiness | 主网进入条件 | `not_mainnet_grade` 直到所有跨域条件具备。 |
| signer custody | Signer custody 与治理真值 | preview bootstrap 不得成为 production custody。 |
| governance signer | Signer custody 与治理真值 | registry-first、admission、quorum、rotation/revocation。 |
| genesis ceremony | 创世、ceremony 与 QA | `not_mint_ready` 直到 binding、ceremony 与 QA pass。 |

## 实现和验证下钻

- 资产授权：`doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.*`。
- Node/replication 的可恢复签名合同：`doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.*`。
- formal tier、manifest 和 public-testnet checklist：`doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.*` 及 companion runbook。
- registry 操作和 evidence：`scripts/governance-registry-drill.sh`、`scripts/governance-registry-live-drill.sh` 与 `doc/testing/evidence/`。

任何实现改动均须分别由 runtime、blockchain ops、QA 与适用的 product/LiveOps authority 审核；本文不提供实现替代方案或放行判断。
