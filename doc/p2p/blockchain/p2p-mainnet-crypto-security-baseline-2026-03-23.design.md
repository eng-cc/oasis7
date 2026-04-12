# oasis7 主链/共识密码学安全基线评估（设计文档）

- 对应需求文档: `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.project.md`

审计轮次: 1
## 设计目标
- 把“局部签名能力”与“系统级 mainnet-grade 安全”拆开评估，避免 producer 在创世与对外口径上高估当前状态。
- 用统一矩阵描述每个安全面向的当前真值、目标态与 blocker，而不是散落在多个专题里口头引用。

## 评估分层
| 层级 | 说明 | 当前结论 |
| --- | --- | --- |
| `crypto_primitives` | 原语是否合格，如 `ed25519`、`HMAC-SHA256`、固定前缀签名载荷 | `mostly_pass` |
| `transaction_authorization` | 资产动作是否统一走签名交易模型 | `block` |
| `account_and_address_model` | 账户/地址是否已达到成熟钱包/账户抽象基线 | `risk` |
| `network_authorization` | replication/fetch request 是否有签名与 allowlist | `preview_pass` |
| `key_custody` | 私钥是否受生产级 keystore / signer 服务保护 | `block` |
| `governance_signer_model` | 治理 finality signer 是否已外部化、可轮换、可吊销 | `block` |
| `genesis_execution_control` | 创世地址、多签、signer rule 是否完成真实绑定和 QA | `block` |

## 当前真值矩阵
| 面向 | 当前真值 | 依据 | producer 结论 |
| --- | --- | --- | --- |
| Viewer 玩家鉴权 | `ed25519` + 域前缀 + nonce + public key match | `crates/oasis7/src/viewer/auth.rs` | 正向信号，但只覆盖 viewer 控制面 |
| Replication / Fetch 鉴权 | request 签名 + writer allowlist + writer/public_key binding | `crates/oasis7_node/src/replication.rs` | 正向信号，属于网络面 preview hardening |
| 主链 Token transfer submit | 接收未签名 JSON，请求侧只做字段/余额/nonce 预检 | `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs` | 系统级 blocker |
| Consensus action payload | payload envelope 无统一签名交易字段 | `crates/oasis7/src/consensus_action_payload.rs` | 系统级 blocker |
| Main token 账户模型 | `recipient` 是 runtime 内字符串；公钥派生为 `oc:pk:<public_key_hex>` | `crates/oasis7/src/runtime/main_token.rs` | 可运行，但不等于成熟外部钱包地址体系 |
| 节点 keypair | 自动生成并明文写入 `config.toml` | `crates/oasis7/src/bin/oasis7_chain_runtime/node_keypair_config.rs` | preview convenience，不算生产级 keystore |
| 治理 finality signer | 仍存在 deterministic local seed signer 路径 | `crates/oasis7/src/runtime/world/governance.rs` | local/test convenience，不算生产治理 signer |
| 创世执行控制 | freeze sheet 仍有 `TBD_BEFORE_MINT` / `pending_binding` | `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md` | not ready for mint |

## 结论规则
- 规则-1: 只要 `transaction_authorization = block`，总 verdict 必须为 `not_mainnet_grade`。
- 规则-2: 只要 `key_custody = block` 或 `governance_signer_model = block`，最多只能宣称 `crypto-safe-enough-for-preview`。
- 规则-3: 只要 `genesis_execution_control != pass`，不得宣称 `mint_ready`。
- 规则-4: 网络面 `preview_pass` 不得覆盖资产面 `block`。

## 路线优先级
1. `P0`: 新开“主链 Token 签名交易鉴权”专题，统一 `TransferMainToken/ClaimMainTokenVesting/InitializeMainTokenGenesis/DistributeMainTokenTreasury` 的签名提交模型。
2. `P1`: 新开“生产级 signer/keystore”专题，去掉明文 `config.toml` 生产依赖，补 signer rotation/revocation/external signer path。
3. `P1`: 新开“治理 finality signer 外部化”专题，替换 deterministic local seed 路径。
4. `P2`: 在上述收口后，再执行创世 recipient/controller 真实绑定与 signer ceremony QA。

## 对外口径
- 当前允许：`limited playable technical preview`、`有基本签名与网络鉴权硬化`、`原语层已采用 ed25519/HMAC`.
- 当前禁止：`对标主流公链安全`、`mainnet-grade`、`创世执行已达生产级`.
