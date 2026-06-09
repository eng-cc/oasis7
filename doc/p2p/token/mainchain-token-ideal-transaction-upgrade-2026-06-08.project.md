# oasis7 主链理想化交易升级方案（项目管理文档）

- 对应需求文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.prd.md`
- 对应设计文档: `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] ideal-transaction-doc-freeze (PRD-P2P-ITX-001/002/003) [test_tier_required]: 新建“主链理想化交易升级方案”专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪。 Trace: .pm/tasks/task_5ea9d9779665405face4c4984c056613.yaml
- [x] ideal-transaction-v2-context (PRD-P2P-ITX-001) [test_tier_required]: 将 `chain_id`、`tx_version`、`tx_type`、`valid_until_unix_ms` 接入 transfer submit、签名 payload、node-local re-verification、explorer 与 launcher surfaces。 Trace: .pm/tasks/task_5ea9d9779665405face4c4984c056613.yaml
- [x] ideal-transaction-asset-metadata (PRD-P2P-ITX-001/002) [test_tier_required]: 补齐 `asset_id`、`memo`、`network_id`，并让 v2 context 触发路径与签名验证保持一致。 Trace: .pm/tasks/task_5ea9d9779665405face4c4984c056613.yaml
- [x] ideal-transaction-live-chain-status (PRD-P2P-ITX-001) [test_tier_required]: 将 `chain_id` / `network_id` 来源切换为 live runtime chain status only，不保留 launcher-local fallback。 Trace: .pm/tasks/task_5ea9d9779665405face4c4984c056613.yaml
- [x] ideal-transaction-fee-quote-metadata (PRD-P2P-ITX-001/002) [test_tier_required]: 补齐 `application_payload_hash`、`max_fee`、`fee_asset_id`、`client_request_id`，覆盖 submit/explorer/launcher/transfer_auth 闭环。 Trace: .pm/tasks/task_5ea9d9779665405face4c4984c056613.yaml
- [x] ideal-transaction-phase-two-scope (PRD-P2P-ITX-003) [test_tier_required]: 收口 scope，确认 `fee_payer`、`sponsor`、`priority_fee` 与 real fee debit 不属于当前字段补齐任务，降级为未来可选 Phase 2+ fee/auth execution model。 Trace: .pm/tasks/task_5ea9d9779665405face4c4984c056613.yaml

## 当前完成字段
| 字段 | 当前结论 |
| --- | --- |
| `chain_id` | implemented; live runtime chain status only |
| `network_id` | implemented; live runtime chain status only |
| `tx_version` | implemented |
| `tx_type` | implemented |
| `valid_until_unix_ms` | implemented with expiry validation |
| `asset_id` | implemented |
| `memo` | implemented |
| `application_payload_hash` | implemented |
| `max_fee` | implemented metadata-only with `> 0` validation |
| `fee_asset_id` | implemented metadata-only; required with `max_fee` |
| `client_request_id` | implemented metadata-only |

## TODO / 后续边界
- Future optional: Phase 2+ fee/auth execution model. 若未来确实需要真实 fee economics，再单独立项设计 `fee_payer`、`sponsor`、`priority_fee`、真实扣费、treasury/burn settlement、mempool priority policy 与 explorer fee events。
- Future optional: Phase 2+ sponsor policy. 定义 sponsor authorization、allowance、revocation、anti-abuse、audit trail 后再考虑字段落地。
- Future optional: Phase 2+ consensus/client idempotency. 如需 `client_request_id` 具备 consensus-level dedupe，另行定义去重窗口、状态存储与 replay 语义；当前仅为签名/展示 metadata。

## 依赖
- `.pm/tasks/task_5ea9d9779665405face4c4984c056613.execution.md`
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- `doc/p2p/README.md`
- `crates/oasis7/src/runtime/events.rs`
- `crates/oasis7/src/consensus_action_payload.rs`
- `crates/oasis7_node/src/node_runtime_core.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_support.rs`
- `crates/oasis7_client_launcher/src/transfer_auth.rs`

## 验收命令（本轮）
- `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_chain_runtime -p oasis7_client_launcher -p oasis7 --bin oasis7_web_launcher`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher transfer_auth -- --nocapture`
- `env -u RUSTC_WRAPPER CARGO_INCREMENTAL=0 cargo test -p oasis7 --bin oasis7_chain_runtime transfer_submit -- --nocapture`
- `rg -n "chain_id|network_id|tx_version|tx_type|valid_until_unix_ms|asset_id|memo|application_payload_hash|max_fee|fee_asset_id|client_request_id|fee_payer|sponsor|priority_fee|real fee debit|metadata-only|Phase 2" doc/p2p/README.md doc/p2p/prd.index.md doc/p2p/prd.md doc/p2p/project.md doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.prd.md doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.design.md doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.project.md`
- `./scripts/doc-governance-check.sh` if the full-corpus gate returns in the current environment; otherwise carry the documented residual risk and targeted-doc-check evidence
- `git diff --check`

## 状态
- 当前阶段: Phase 1 metadata-only rollout complete; closeout in progress.
- 下一步: 运行文档门禁与最终验证，完成 task closeout、repo-owned pre-PR review、commit 与 PR/merge 主链。
- 最近更新: 2026-06-09
