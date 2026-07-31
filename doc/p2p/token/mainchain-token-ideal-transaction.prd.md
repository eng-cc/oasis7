# oasis7 主链理想交易合同

- 对应设计文档: `doc/p2p/token/mainchain-token-ideal-transaction.design.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

## 1. Executive Summary

- **Authority:** 本三件套是主链 Token signed transaction metadata envelope 的稳定专业权威；它吸收已删除的 dated ideal-transaction source triad，并承接其 `PRD-P2P-ITX-001/002/003` 追溯身份。
- **Current contract:** `TransferMainToken` 已完成 signed metadata 的 Phase 1 闭环。真实 fee debit、sponsor、fee payer 与 priority fee 仍不属于当前实现，不能据此作 mainnet-grade economics、钱包/keystore 或公开交易执行宣称。
- **Success criteria:** runtime、node-local re-verification、submit、explorer、launcher 与 transfer auth helper 对同一字段集合和签名 payload 一致；客户端只从 live runtime chain status 获取 chain/network identity。

## 2. User Experience & Functionality

| Field | Current status | Contract |
| --- | --- | --- |
| `chain_id` / `network_id` | implemented | Signed metadata；只能来自 live runtime chain status，缺失即 launcher submit fail-fast，不得 fallback。 |
| `tx_version` / `tx_type` | implemented | v2 context 使用 `2` / `asset_transfer`。 |
| `valid_until_unix_ms` | implemented | 非正或已过期即 reject。 |
| `asset_id` / `memo` / `application_payload_hash` | implemented | 均为 signed/displayed metadata；主链 Token 当前 asset 为 `main_token`。 |
| `max_fee` / `fee_asset_id` | implemented metadata-only | 报价槽位，`max_fee=0` 或缺 fee asset 即 reject；不扣费、不结算。 |
| `client_request_id` | implemented metadata-only | signed audit/idempotency metadata，不承诺 consensus-level dedupe。 |

All implemented fields are included in `TransferMainToken` signing payload and in node-local signature rebuild/reverification. Explorer p0/p1 and launcher surface the submitted metadata; fee quote display must remain explicitly metadata-only.

### Canonical request shape

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

## 3. AI System Requirements (If Applicable)

不适用。本专题不引入 AI 模型能力；它只定义链上 Token transfer 的签名 metadata 合同与可审计展示边界。

## 4. Technical Specifications

### Explicit non-goals and future gate

`fee_payer`, `sponsor`, `priority_fee`, real fee debit, fee-market, mempool priority, multi-party authorization, sponsor allowance/revocation, treasury/burn settlement, explorer fee events and consensus dedupe are **not** current features. A Phase 2+ fee/auth execution authority must define authorization, affordability, debit/settlement, abuse controls, audit events, priority policy and replay semantics before any of them may be implemented or claimed.

### Integration and verification

Primary implementation surfaces are `crates/oasis7/src/runtime/events.rs`, `crates/oasis7/src/consensus_action_payload.rs`, `crates/oasis7_node/src/node_runtime_core.rs`, chain-runtime transfer/explorer APIs, `oasis7_web_launcher`, and client-launcher `transfer_auth` / explorer support.

Required targeted evidence:

```bash
env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_chain_runtime -p oasis7_client_launcher -p oasis7 --bin oasis7_web_launcher
env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher transfer_auth -- --nocapture
env -u RUSTC_WRAPPER CARGO_INCREMENTAL=0 cargo test -p oasis7 --bin oasis7_chain_runtime transfer_submit -- --nocapture
```

## 5. Risks & Roadmap

Phase 1 metadata-only signed transaction closure is complete. The principal risk is semantic overclaim: quote metadata must never be presented as charged fees, and future payer/sponsor/priority fields must never be presented as execution support. Phase 2+ remains separately scoped as described above.

## 6. Validation & Decision Record

| ID | Decision |
| --- | --- |
| `PRD-P2P-ITX-001` | Sign every implemented transaction metadata field and use the same shape in node re-verification. |
| `PRD-P2P-ITX-002` | Use live chain status without launcher-local identity fallback. |
| `PRD-P2P-ITX-003` | Keep fee execution, sponsor semantics and priority policy as separately designed Phase 2+ work. |
