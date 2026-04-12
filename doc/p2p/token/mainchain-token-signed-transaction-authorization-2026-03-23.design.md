# oasis7 主链 Token 签名交易鉴权（设计文档）

- 对应需求文档: `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.project.md`

审计轮次: 2
## 设计目标
- 在不重做整个资产动作协议的前提下，先关闭当前公开 `transfer submit` 面的未签名提交漏洞。
- 复用现有 `ed25519` 原语与 `oc:pk:<public_key_hex>` 账户派生规则，把请求级鉴权前置到 HTTP submit 入口。
- 把 signed transaction model 上提到 shared `ConsensusActionPayloadEnvelope` / `NodeRuntime` 提交层，避免未来新 submit surface 再次绕过。
- 明确这是统一 signed transaction model 的推进切片；`STRAUTH-2B1` 只做 controller slot binding，`STRAUTH-2B2` 再把 genesis/treasury 升级到 threshold signer allowlist enforcement，但仍不伪造 ceremony 已完成。
- 在 `STRAUTH-3A` 把 `oasis7_client_launcher` 的 Web/native 转账窗口补到“本地产签再提交”，并让 `oasis7_web_launcher` 为 wasm 注入受信本地 signer bootstrap。

## 请求契约
| 字段 | 含义 | 规则 |
| --- | --- | --- |
| `from_account_id` | 主链转出账户 | 必须等于 `oc:pk:<normalized_public_key_hex>` |
| `to_account_id` | 主链转入账户 | 沿用现有账户格式规则 |
| `amount` | 转账数量 | `> 0` |
| `nonce` | 转账 nonce | `> 0`，通过现有 runtime 规则做 anti-replay |
| `public_key` | `ed25519` 公钥 hex | 32-byte，规范化为小写 |
| `signature` | 签名 hex | transfer 当前沿用固定域前缀 + 64-byte `ed25519` 签名 |

## Shared Payload Auth Envelope
| 字段 | 说明 |
| --- | --- |
| `auth.type` | 当前固定为 `main_token_action` |
| `auth.data.account_id` | 资产动作声明的账户/控制者标识 |
| `auth.data.public_key` | `ed25519` 公钥 |
| `auth.data.signature` | 固定域前缀签名 |

- `ConsensusActionPayloadEnvelope` 对非主链 Token action 继续允许 `auth=None`。
- 对 `TransferMainToken / ClaimMainTokenVesting / InitializeMainTokenGenesis / DistributeMainTokenTreasury`，`NodeRuntime` 必须要求 `auth=Some(main_token_action)`。

## Controller Slot Registry（STRAUTH-2B1）
| registry item | current source | meaning |
| --- | --- | --- |
| `genesis_controller_account_id` | `NodeConfig.main_token_controller_binding` | `InitializeMainTokenGenesis` 唯一允许的治理 controller slot |
| `treasury_bucket_controller_slots[bucket_id]` | `NodeConfig.main_token_controller_binding` | 每个 treasury bucket 对应的唯一 controller slot |

- `STRAUTH-2B1` 先把 controller slot registry 落在 `NodeConfig`，供 `NodeRuntime` submit-layer 阻断使用。
- 这不是最终 on-chain / world-state 治理配置，只是把“任意 controller label”升级成“正式 slot binding”。
- `STRAUTH-2B2` 再补 slot -> signer allowlist / threshold / ceremony。

## Controller Signer Policy（STRAUTH-2B2）
| policy item | current source | meaning |
| --- | --- | --- |
| `controller_signer_policies[controller_account_id].threshold` | `NodeConfig.main_token_controller_binding` | controller slot 当前要求的最小唯一 signer 数 |
| `controller_signer_policies[controller_account_id].allowed_public_keys` | `NodeConfig.main_token_controller_binding` | 当前允许为该 controller slot 签名的 ed25519 公钥集合 |

- `STRAUTH-2B2` 继续沿用 `NodeConfig` 作为 submit-layer source of truth。
- 当前只收口到“本地配置 allowlist + threshold enforcement”，不触碰 ceremony 自动化，也不把数据迁进 world-state。
- 若 policy 缺失或 allowlist 为空，genesis/treasury submit-layer 直接拒绝。

## Canonical Payload（shared）
- 每个主链 Token action 都使用同一签名外框；对于 threshold proof，每个参与 signer 都对下面这份 canonical payload 单独签名一次：

```json
{
  "version": 1,
  "operation": "<action_operation>",
  "account_id": "<authorized_account_or_controller>",
  "public_key": "<signer_public_key_hex>",
  "action": { "...runtime action json..." }
}
```

- `operation` 与签名前缀按 action 区分，用于域隔离。
- transfer HTTP 入口继续沿用请求级校验，再把已有签名材料写入 shared payload auth envelope。
- `oasis7_client_launcher` native 侧直接复用 Rust signer helper；wasm 侧必须复刻同一份 canonical JSON 与前缀，避免浏览器端与 runtime 合约漂移。
- 对 `TransferMainToken`，wasm 侧签名前的 `action` JSON 必须与 runtime helper 的实际序列化字节一致：
  - `action` 外层字段顺序固定为 `data` 再 `type`
  - `data` 内字段顺序固定为 `amount/from_account_id/nonce/to_account_id`
  - 该约束已由 `transfer_auth` 定向回归锁定，防止浏览器端因为字段顺序变化再次产出 `invalid_signature`

## Shared Auth Proof Shape（STRAUTH-2B2）
| scheme | required fields | usage |
| --- | --- | --- |
| `ed25519` | `account_id/public_key/signature` | transfer / claim / 允许单签的 controller policy |
| `threshold_ed25519` | `account_id/threshold/participant_signatures[]` | genesis / treasury controller proof |

- `participant_signatures[]` 的每一项都包含 `public_key` 与 `signature`。
- `threshold_ed25519` proof 中每个参与 signer 都必须使用自己的 `public_key` 生成同一 action/account_id 的 canonical payload。

## 提交层校验规则
| action | 提交层规则 | 当前安全结论 |
| --- | --- | --- |
| `TransferMainToken` | `auth.account_id == from_account_id` 且必须等于 `oc:pk:<public_key_hex>` | 账户绑定成立 |
| `ClaimMainTokenVesting` | `auth.account_id == beneficiary`；若 beneficiary 为 `oc:pk:`，需校验公钥派生；若为 `protocol:*` 等命名账户，只要求签名与 account_id 一致 | 已签名化，但命名控制账户的真实 controller binding 仍待治理专题 |
| `InitializeMainTokenGenesis` | 必须带 signed controller metadata，`auth.account_id` 命中 `genesis_controller_account_id`，并通过该 slot 的 signer allowlist / threshold 校验 | 已完成代码级 signer policy enforcement，但真实创世 ceremony 仍待治理专题 |
| `DistributeMainTokenTreasury` | 必须带 signed controller metadata，`auth.account_id` 命中 `bucket_id -> controller slot`，并通过该 slot 的 signer allowlist / threshold 校验 | 已完成代码级 signer policy enforcement，但真实 treasury governance ceremony 仍待治理专题 |

## 首切片边界
| 资产动作 | 当前状态 | 原因 |
| --- | --- | --- |
| `TransferMainToken` 公开 HTTP submit | `implemented` | 当前唯一公开资产提交面，已完成请求级签名鉴权 |
| Shared payload auth envelope | `implemented_in_this_slice` | 是所有未来 submit surface 的汇合点 |
| `ClaimMainTokenVesting` payload submit gate | `implemented_in_this_slice` | 已有 beneficiary，可先纳入 shared envelope |
| `InitializeMainTokenGenesis` payload submit gate | `implemented_in_this_slice` | 先要求 signed controller metadata，再留待治理绑定 |
| `DistributeMainTokenTreasury` payload submit gate | `implemented_in_this_slice` | 先要求 signed controller metadata，再留待治理绑定 |
| Governance controller slot binding | `implemented_in_this_slice` | 通过 `NodeConfig` registry 收紧 controller label |
| Governance signer allowlist / threshold | `implemented_in_this_slice` | 通过 `NodeConfig` policy 收紧 controller signer 集合 |
| Governance ceremony / external signer | `pending` | 需要 producer/QA/治理专题联审 |

## 错误码约定
| error_code | 触发条件 |
| --- | --- |
| `invalid_request` | JSON 缺字段、字段为空、公钥格式非法、金额/nonce 非法 |
| `invalid_signature` | 签名前缀不对、签名长度非法、签名验签失败 |
| `account_auth_mismatch` | `from_account_id` 不是该公钥派生账户 |
| `missing_main_token_auth` | token runtime action 在 payload 层缺 auth proof |
| `insufficient_balance` | 通过鉴权后余额不足 |
| `nonce_replay` | 通过鉴权后 nonce 不满足递增规则 |

## 兼容性与后续
- `oasis7_web_launcher` 除透传 transfer 新字段外，还需要在服务静态 HTML 时注入 `__OASIS7_VIEWER_AUTH_ENV`，让 wasm 转账窗口能读取本地 signer bootstrap。
- `oasis7_client_launcher` 的 Web/native 转账窗口现在都必须在本地生成签名后再提交；这仍是 trusted local bootstrap，不是钱包托管或生产级 keystore。
- 由于 launcher Web UI 当前是 canvas-only，`STRAUTH-3B` 额外引入最小 wasm test hook：
  - `window.__OASIS7_LAUNCHER_TEST_QUEUE`
  - `window.__OASIS7_LAUNCHER_TEST_STATE`
  - 该钩子只用于 agent-browser/QA 自动化驱动与状态镜像，不改变 runtime 权限边界，也不替代正式玩家控制面。
- `genesis/treasury` 在 `STRAUTH-2B2` 完成后，会进入 shared envelope + controller signer allowlist / threshold enforcement；但外部 signer、ceremony freeze、HSM/KMS 仍需后续专题完成。
