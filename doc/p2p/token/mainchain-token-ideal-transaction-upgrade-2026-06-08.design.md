# oasis7 主链理想化交易升级方案（设计文档）

- 对应需求文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.prd.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.project.md`

审计轮次: 1

## 设计目标
- 把主链 Token transfer 从裸参数提交升级为具备链身份、资产、版本、有效期、费用报价和客户端审计标识的 signed transaction metadata envelope。
- 让 runtime、node-local re-verification、explorer、launcher 与 transfer auth helper 对同一字段集合保持一致。
- 明确 Phase 1 只实现 metadata-only 字段闭环；真实扣费、sponsor、fee payer 与 priority fee 不在本阶段偷偷落地。

## Phase 1 字段集合
| 字段 | 类型 | 设计语义 |
| --- | --- | --- |
| `chain_id` | `Option<String>` | 运行中链身份，来自 live runtime chain status。 |
| `network_id` | `Option<String>` | 运行中网络身份，来自 live runtime chain status。 |
| `tx_version` | `Option<u8>` | 交易 metadata 协议版本，目前 v2。 |
| `tx_type` | `Option<String>` | 交易类型，目前 `asset_transfer`。 |
| `valid_until_unix_ms` | `Option<i64>` | 客户端请求有效期，runtime 做过期拒绝。 |
| `asset_id` | `Option<String>` | 资产标识，目前主链 Token 为 `main_token`。 |
| `memo` | `Option<String>` | 用户/客户端可读备注，签名覆盖。 |
| `application_payload_hash` | `Option<String>` | 应用层 payload commitment。 |
| `max_fee` | `Option<u64>` | 费用报价上限 metadata；不扣费。 |
| `fee_asset_id` | `Option<String>` | 费用报价资产 metadata；`max_fee` 存在时必填。 |
| `client_request_id` | `Option<String>` | 客户端审计/幂等标识 metadata；不承诺 consensus dedupe。 |

## 当前交易 JSON 草案
```json
{
  "from_account_id": "oc:pk:<public_key_hex>",
  "to_account_id": "oc:pk:<recipient_public_key_hex>",
  "amount": 42,
  "nonce": 7,
  "public_key": "<public_key_hex>",
  "signature": "<signature_hex>",
  "chain_id": "oasis7-public-testnet",
  "network_id": "public_testnet",
  "tx_version": 2,
  "tx_type": "asset_transfer",
  "valid_until_unix_ms": 1790000000000,
  "asset_id": "main_token",
  "memo": "optional note",
  "application_payload_hash": "blake3:<hex>",
  "max_fee": 10,
  "fee_asset_id": "main_token",
  "client_request_id": "client-generated-id"
}
```

## 签名域
- `TransferMainToken` 的 signing payload 必须包含所有已实现 metadata 字段。
- node-local signature rebuild 必须使用同一字段集合，避免 runtime 接受而 node re-verification 拒绝。
- `chain_id` / `network_id` 必须由 live runtime chain status 提供；没有 live identity 时不签 fallback。

## 校验规则
| 条件 | 结果 |
| --- | --- |
| v2 context 字段存在但 `tx_version != 2` | reject |
| v2 context 字段存在但 `tx_type != asset_transfer` | reject |
| `valid_until_unix_ms <= 0` | reject |
| `valid_until_unix_ms < now` | reject |
| `max_fee == 0` | reject |
| `max_fee` 存在但 `fee_asset_id` 缺失 | reject |
| live `chain_id` / `network_id` 缺失 | launcher submit fail-fast |

## Explorer / Launcher 回执
- Explorer p0/p1 transaction item must surface the signed metadata fields that were submitted.
- Launcher explorer views are implemented to display the same metadata so operators can audit the request without raw JSON reconstruction; the freshest verification in this task turn is the targeted auth/runtime/explorer regression set plus code-path inspection, not a new dedicated launcher-display assertion for every final field.
- `max_fee` / `fee_asset_id` display must be described as fee quote metadata only.

## Phase 2+ 不在本阶段做的事
| Future field/model | 为什么不作为当前字段补齐 |
| --- | --- |
| `fee_payer` | 当前 auth proof 绑定单一 `account_id == from_account_id`；真实 payer 分离需要 multi-party auth 和 payer debit。 |
| `sponsor` | 需要赞助授权、额度、撤销、反滥用和审计事件。 |
| `priority_fee` | 需要 mempool 排序、费用竞价、执行扣费或结算策略。 |
| real fee debit | 需要 balance affordability、fee settlement、treasury/burn policy 和 explorer fee event。 |

## Rollout
- Phase 1 complete: metadata-only fields are implemented end-to-end across runtime/node/launcher/explorer surfaces, with fresh targeted `cargo check`, `transfer_auth`, runtime `transfer_submit`, and whitespace verification recorded for this task.
- Phase 2 optional: open a separate fee/auth execution model only when product/runtime explicitly needs anti-spam pricing, sequencer incentives, sponsored transactions or priority policy.

## 验证
- Required checks:
  - `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_chain_runtime -p oasis7_client_launcher -p oasis7 --bin oasis7_web_launcher`
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher transfer_auth -- --nocapture`
  - `env -u RUSTC_WRAPPER CARGO_INCREMENTAL=0 cargo test -p oasis7 --bin oasis7_chain_runtime transfer_submit -- --nocapture`
  - `./scripts/doc-governance-check.sh` when the full-corpus doc gate completes in the current environment; otherwise record the failure-to-complete and run targeted doc checks for the touched topic/module surfaces
  - `git diff --check`
