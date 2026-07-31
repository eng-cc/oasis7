# oasis7 主链理想交易合同（设计）

- 对应需求文档: `doc/p2p/token/mainchain-token-ideal-transaction.prd.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

## Design authority

This stable design absorbs the dated 2026-06-08 ideal-transaction design. Its purpose is a single signed metadata envelope shared by runtime, node-local re-verification, explorer, launcher and transfer-auth surfaces. It does not change consensus, custody, wallet, or fee-economics mechanisms.

## Phase 1 field and validation model

| Field group | Design rule |
| --- | --- |
| Chain identity | `chain_id` and `network_id` are live-runtime sourced, signed, and never launcher-local fallbacks. |
| Context | `tx_version=2` and `tx_type=asset_transfer` are required whenever v2 context is present. |
| Expiry | `valid_until_unix_ms` is signed and runtime rejects non-positive or expired values. |
| Asset/application | `asset_id`, `memo`, and `application_payload_hash` are signed and displayable. |
| Quote | `max_fee` is non-zero when present and requires `fee_asset_id`; both are metadata only, with no debit. |
| Client audit | `client_request_id` is signed audit/idempotency metadata only, not consensus dedupe. |

The signing payload and node-local signature rebuild use the exact same implemented fields. A submission whose live chain identity is unavailable fails before signing; structural validation rejects invalid v2 context and incomplete quote metadata.

## Surface contract

- Runtime action: `Action::TransferMainToken` carries the implemented metadata.
- Node: locally rebuilds/reverifies the same signed payload; no node-local bypass.
- Explorer/launcher: display submitted metadata for audit without requiring raw JSON reconstruction.
- Fees: `max_fee` and `fee_asset_id` are labelled quote metadata, never evidence of fee charging.

## Phase 2+ design boundary

No field may be added merely as an optional string. `fee_payer` requires separate payer authorization and debit; `sponsor` requires authorization, allowance/limits, revocation, abuse controls and audit; `priority_fee` requires mempool and execution ordering policy; real fee debit requires affordability, settlement, treasury/burn policy and explorer events. These require a separately approved fee/auth execution model.

## Rollout and residual risk

Phase 1 is the implemented metadata-only closure. The residual risk is semantic, not an operational shortcut: presenting fee quote fields as charged fees, or future fields as supported sponsor/priority execution, is false. This design cannot be used as mainnet readiness, external-custody, or token-economics authority.
