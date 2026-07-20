# oasis7 主链 mainnet-grade readiness 硬化路线（设计文档）

> 历史状态说明：本文的阶段表与 MAINNET-4 规则保存 2026-03-23 设计基线，不是当前公开状态权威。当前状态以根 `README.md` 为准，现行网络层级与测试结论以对应专业文档和证据为准。

- 对应需求文档: `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.project.md`

审计轮次: 1
## 设计目标
- 把 `STRAUTH-3` 之后仍未闭环的系统级安全问题，统一收成 producer 可以逐项推进的 readiness gate。
- 明确当前阶段虽然已是 `crypto-hardened preview`，但距离 `mainnet-grade candidate` 还差 signer custody、治理 signer、创世 freeze/ceremony 三个系统面向。

## 当前阶段定义
| 阶段 | 含义 | 当前结论 |
| --- | --- | --- |
| `limited_playable_technical_preview` | 游戏可有限试玩，但仍以技术验证为主 | `pass` |
| `crypto_hardened_preview` | 公开资产动作已进入 signed transaction gating，Web/native 有基本签名闭环 | `pass` |
| `mainnet_grade_candidate` | signer custody、governance signer、genesis freeze/ceremony 全部达标，可进入更高安全口径复评 | `block` |

## Readiness Gate 矩阵
| Gate | 当前真值 | 目标态 | 当前 blocker |
| --- | --- | --- | --- |
| `MAINNET-1 signer_custody` | transfer/UI signer 仍来源于 trusted local env/config bootstrap；节点 keypair 仍可落明文 `config.toml` | 生产路径切到 managed keystore 或 external signer，具备 rotation/revocation/audit | 无生产级 signer custody |
| `MAINNET-2 governance_signer` | genesis/treasury 已有本地 controller signer policy，但仍停留在 `NodeConfig`，governance finality 仍含 local seed 路径 | 治理 signer 外部化，正式支持 rotation/revocation/failover | deterministic seed 与本地配置真值未退出 |
| `MAINNET-3 genesis_freeze_ceremony` | freeze sheet 仍含 `TBD_BEFORE_MINT` / `pending_binding`，无正式 ceremony QA bundle | recipient/controller/signer policy 全冻结，并产出 ceremony + QA 证据 | 创世真值和执行证据未冻结 |
| `MAINNET-4 claims_re_evaluation` | 对外口径仍需限制在 preview 档 | 仅在前三个 gate 全绿后重评估是否可提升表述 | 当前不允许升级 public claims |

## 依赖顺序
1. `MAINNET-1` 先冻结 signer custody 目标态，避免后续治理与创世继续绑定在临时 signer 源上。
2. `MAINNET-2` 再把治理 finality signer 和 controller signer 真值迁出本地 seed/config。
3. `MAINNET-3` 基于稳定 signer 源冻结创世 recipient/controller/signer policy，并执行 ceremony/QA。
4. `MAINNET-4` 最后做阶段复评和 public claims policy 收口。

## Gate 完成定义
| Gate | 最小完成定义 | 不算完成的情况 |
| --- | --- | --- |
| `MAINNET-1` | `managed signer/external signer`、`rotation`、`revocation`、`audit trail`、owner/QA 认可 | 继续依赖本地 env/config bootstrap、明文 `config.toml` |
| `MAINNET-2` | 治理 finality signer 与 controller signer 均具备 externalized truth、rotation、revocation、failover | 只做本地 allowlist/threshold，不退出 local seed/config |
| `MAINNET-3` | freeze sheet 清零 TBD，ceremony checklist 执行，QA 产出 `pass/block`，证据 bundle 可审计 | 只口头确认地址/公钥，或无 QA 结论 |
| `MAINNET-4` | producer 根据前三个 gate 与 QA 结果重评估，并明确 public claims policy | 仅凭工程感觉升级安全表述 |

## 对外口径控制
- 当前允许：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `signed transaction model is in place for exposed main-token paths`
- 当前禁止：
  - `mainnet-grade`
  - `mainstream public-chain-grade security`
  - `production mint ready`
