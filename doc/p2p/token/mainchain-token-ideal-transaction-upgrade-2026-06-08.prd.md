# oasis7 主链理想化交易升级方案（2026-06-08）

- 对应设计文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.design.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.project.md`

审计轮次: 1

## 目标
- 冻结主链 Token transfer 从裸参数提交升级到 signed transaction metadata envelope 的目标态。
- 明确当前 Phase 1 已完成 metadata-only 字段闭环，避免把未来 fee/auth execution model 误认为当前实现。
- 给 runtime、node、launcher、explorer 与 QA 提供同一字段集合和验收口径。

## 范围
- 覆盖 `TransferMainToken` 当前已实现的交易 metadata 字段、签名覆盖、展示传播、结构校验和测试边界。
- 覆盖未来可选 `fee_payer`、`sponsor`、`priority_fee`、real fee debit 的 Phase 2+ 立项边界。
- 不覆盖真实 fee-market、赞助交易执行、mempool priority、生产级钱包/keystore 或 mainnet-grade 经济安全宣称。

## 接口 / 数据
- PRD 主入口: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.prd.md`
- 设计文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.design.md`
- 项目管理文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.project.md`
- 当前 runtime action: `Action::TransferMainToken`
- 当前实现字段: `chain_id`、`network_id`、`tx_version`、`tx_type`、`valid_until_unix_ms`、`asset_id`、`memo`、`application_payload_hash`、`max_fee`、`fee_asset_id`、`client_request_id`

## 里程碑
- M1 (2026-06-08): 完成理想化交易模型 topic 文档冻结与 p2p 模块入口映射。
- M2 (2026-06-09): 完成 Phase 1 metadata-only 字段在 transfer submit / signing / node re-verification / explorer / launcher 的闭环实现与定向验证。
- M3: 若需要真实 fee economics，另开 Phase 2+ fee/auth execution model，而不是在 Phase 1 中继续追加误导性字段。

## 风险
- 将 `max_fee + fee_asset_id` 误读成真实已扣费会误导运营和用户。
- 将 `fee_payer` / `sponsor` / `priority_fee` 当作普通字段追加，会制造“看起来支持、实际没有授权/扣费/优先级语义”的 API。
- 若 `client_request_id` 被误读成 consensus-level dedupe，会高估当前幂等能力；本阶段只承诺签名/展示 metadata。

## 1. Executive Summary
- Problem Statement: 当前主链 Token transfer 已具备签名鉴权，但长期交易模型仍容易被理解成裸 `from/to/amount/nonce` 参数，缺少统一交易对象、链身份、资产、有效期、费用报价、客户端幂等标识与 explorer 回执字段的目标态。
- Proposed Solution: 将主链 Token transfer 升级为签名覆盖的 transaction metadata envelope。当前实现阶段先完成 metadata-only 字段闭环；真实 fee debit、sponsor、fee payer 与 priority fee 不混入本阶段，作为 Phase 2+ fee/auth execution model 单独立项。
- Success Criteria:
  - SC-1: `TransferMainToken` 已补齐的交易 metadata 字段必须进入 request、签名 payload、runtime action、node-local re-verification、submit record/tracker、explorer p0/p1 与 launcher 展示闭环。
  - SC-2: `chain_id` / `network_id` 必须来自 live runtime chain status，提交端不得使用 launcher-local fallback。
  - SC-3: `max_fee + fee_asset_id` 只作为签名/展示/结构校验的费用报价槽位，不声明真实扣费已经完成。
  - SC-4: `fee_payer`、`sponsor`、`priority_fee` 不作为当前必补字段；若未来需要，必须单独设计 multi-party auth、fee debit/settlement、sponsor authorization 与 priority/mempool policy。

## 2. User Experience & Functionality
- User Personas:
  - `runtime_engineer`: 需要稳定的签名字段集合，避免 runtime/node/launcher 对同一交易 payload 理解不一致。
  - `viewer_engineer`: 需要 explorer 和 launcher 能展示交易上下文，而不是只展示转出、转入和数量。
  - `qa_engineer`: 需要可以用定向回归证明新增字段被签名、传播和展示，而不是只在某个 API shape 上出现。
  - `producer_system_designer`: 需要区分当前 metadata-only 交易升级与未来 fee-market/sponsored-transaction 执行模型，避免对外过度承诺。
- User Stories:
  - PRD-P2P-ITX-001: As a runtime engineer, I want transfer transaction metadata to be signed and reverified consistently, so that the same submitted transaction is interpreted identically by runtime, node and client surfaces.
  - PRD-P2P-ITX-002: As a viewer engineer, I want explorer and launcher displays to include chain, network, asset, memo, fee quote and client request metadata, so that operators can audit a transfer without reconstructing raw payloads.
  - PRD-P2P-ITX-003: As a producer system designer, I want fee execution fields separated from metadata fields, so that future fee-market work is not falsely implied by present API fields.
- Critical User Flows:
  1. Flow-P2P-ITX-001: `launcher reads live runtime chain status -> builds signed transfer request -> runtime verifies signature -> node locally re-verifies action payload -> explorer surfaces same metadata`
  2. Flow-P2P-ITX-002: `client provides max_fee + fee_asset_id -> runtime structurally validates quote metadata -> explorer/launcher display quote -> no fee debit occurs in this phase`
  3. Flow-P2P-ITX-003: `future fee-market request emerges -> open Phase 2+ fee/auth execution model -> define payer/sponsor/priority semantics before adding execution fields`
- Functional Specification Matrix:
| Field | Current status | Behavior |
| --- | --- | --- |
| `chain_id` | implemented | Signed metadata sourced from live runtime chain status only. |
| `network_id` | implemented | Signed metadata sourced from live runtime chain status only. |
| `tx_version` | implemented | Enables v2 transfer metadata context. |
| `tx_type` | implemented | Must be `asset_transfer` when v2 context fields are present. |
| `valid_until_unix_ms` | implemented | Runtime rejects expired requests and non-positive v2 validity. |
| `asset_id` | implemented | Signed/displayed asset metadata, currently defaults to `main_token`. |
| `memo` | implemented | Signed/displayed memo metadata with normalization and length validation. |
| `application_payload_hash` | implemented | Signed/displayed app payload commitment metadata. |
| `max_fee` | implemented metadata-only | Signed/displayed fee quote upper bound; `0` is rejected. No debit semantics. |
| `fee_asset_id` | implemented metadata-only | Required when `max_fee` is present. No debit semantics. |
| `client_request_id` | implemented metadata-only | Signed/displayed client idempotency/audit metadata. |
| `fee_payer` | future optional | Requires separate payer auth and debit semantics; not current scope. |
| `sponsor` | future optional | Requires sponsor authorization, limits, revocation and audit semantics; not current scope. |
| `priority_fee` | future optional | Requires mempool/priority policy and fee-market execution semantics; not current scope. |
- Acceptance Criteria:
  - AC-1: The implemented 11 fields are present on `Action::TransferMainToken` and all submit/explorer/launcher transfer surfaces that need to carry them.
  - AC-2: Signing payloads include all implemented metadata fields, and node-local signature rebuild/reverification uses the same shape.
  - AC-3: Transfer submit rejects expired `valid_until_unix_ms`, invalid v2 `tx_version`/`tx_type`, `max_fee=0`, and `max_fee` without `fee_asset_id`.
  - AC-4: `chain_id` and `network_id` are supplied from live runtime chain status without fallback.
  - AC-5: `fee_payer` / `sponsor` / `priority_fee` remain documented as future optional Phase 2+ fields, not current TODOs.
- Non-Goals:
  - Do not implement real transfer fee debit in this phase.
  - Do not implement multi-party payer/sponsor authorization in this phase.
  - Do not implement fee-market or mempool priority ordering in this phase.
  - Do not claim mainnet-grade transaction economics from metadata-only fee quote fields.

## 3. Technical Specifications
- Architecture Overview: The current slice extends `TransferMainToken` with signed metadata and propagates it through the existing transfer submit path. Runtime validation remains limited to structural and expiry checks. The transfer amount itself still uses the existing sender-funded token transfer semantics; fee quote fields are not debited or settled.
- Integration Points:
  - `crates/oasis7/src/runtime/events.rs`
  - `crates/oasis7/src/consensus_action_payload.rs`
  - `crates/oasis7_node/src/node_runtime_core.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_support.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/explorer_p0_api.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/explorer_p0_store.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_explorer_p1_api.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher.rs`
  - `crates/oasis7_client_launcher/src/transfer_auth.rs`
  - `crates/oasis7_client_launcher/src/web_api_support.rs`
  - `crates/oasis7_client_launcher/src/explorer_window.rs`
- Edge Cases & Error Handling:
  - If live chain status has not provided `chain_id` / `network_id`, launcher transfer submit fails fast rather than signing fallback identity.
  - If any v2 context field is present, `tx_version` and `tx_type` must be present and match the expected constants.
  - If `max_fee` is present without `fee_asset_id`, runtime rejects the request because the fee quote is not self-describing.
  - If `client_request_id` is present, it is treated as signed audit/idempotency metadata only; no consensus-level dedupe guarantee is implied by this phase.
- Non-Functional Requirements:
  - NFR-P2P-ITX-1: Added metadata must not create unsigned or node-local re-verification bypasses.
  - NFR-P2P-ITX-2: Metadata-only fee fields must not be documented as actual fee charging.
  - NFR-P2P-ITX-3: Local builds use the shared cargo target/cache; when cargo locks are encountered, wait in place and do not create a new compile directory.

## 4. Risks & Roadmap
- Phase 1: Implement metadata-only signed transaction fields, live chain identity sourcing, explorer/launcher display and targeted regression coverage.
- Phase 2+: If product/runtime needs real fee economics, open a separate fee/auth execution model covering payer auth, sponsor authorization, balance checks, debit/settlement, treasury/burn policy, explorer fee events and priority/mempool rules.
- Risks:
  - Risk-1: Treating `max_fee + fee_asset_id` as charged fees would mislead operators and users; docs must consistently call them metadata-only.
  - Risk-2: Adding `fee_payer` or `sponsor` as simple optional strings would create a misleading API that appears to support sponsored transactions without authorization or debit semantics.
  - Risk-3: Adding `priority_fee` without mempool policy would create a field that cannot affect execution order and would be operationally confusing.

## 5. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | Task | Test tier | Verification |
| --- | --- | --- | --- |
| PRD-P2P-ITX-001 | p2p-chain-transfer-submit-client | `test_tier_required` | `cargo check` across runtime/web launcher/client launcher, `transfer_auth`, runtime `transfer_submit`, `git diff --check` |
| PRD-P2P-ITX-002 | p2p-chain-transfer-submit-client | `test_tier_required` | explorer p0/p1 and launcher display regression coverage for metadata propagation |
| PRD-P2P-ITX-003 | p2p-chain-transfer-submit-client | `test_tier_required` | doc/project scope update proving fee execution fields are future optional work |
- Decision Log:
| Decision ID | Selected approach | Rejected alternative | Reason |
| --- | --- | --- | --- |
| DEC-P2P-ITX-001 | Finish Phase 1 as metadata-only signed transaction fields | Continue adding fee_payer/sponsor/priority_fee as current fields | Those fields require execution semantics and would be misleading as plain metadata. |
| DEC-P2P-ITX-002 | Use live runtime chain status for chain identity without fallback | Sign launcher-local fallback chain identity | Mainstream chain transaction context should bind to the running chain, not local launcher assumptions. |
| DEC-P2P-ITX-003 | Keep real fee debit as Phase 2+ | Debit fees opportunistically in transfer submit | Fee debit needs product/runtime policy, settlement events and audit semantics beyond this task. |
