# oasis7 主链理想交易合同（项目管理）

- 对应需求文档: `doc/p2p/token/mainchain-token-ideal-transaction.prd.md`
- 对应设计文档: `doc/p2p/token/mainchain-token-ideal-transaction.design.md`

## 任务拆解（含吸收任务谱系）

- [x] ideal-transaction-doc-freeze (PRD-P2P-ITX-001) [test_tier_required]: the former dated topic is absorbed into this stable authority; historical process remains traceable through Git and GitHub task evidence. Trace: #2735 (task_9d8c13c232d04249b8c3e4dfb709acb2)
- [x] ideal-transaction-v2-context (PRD-P2P-ITX-001) [test_tier_required]: chain id, transaction version/type and validity reached submit, signing, node verification, explorer and launcher. Trace: #2735 (task_9d8c13c232d04249b8c3e4dfb709acb2)
- [x] ideal-transaction-asset-metadata (PRD-P2P-ITX-002) [test_tier_required]: asset, memo and network metadata reached the same closure. Trace: #2735 (task_9d8c13c232d04249b8c3e4dfb709acb2)
- [x] ideal-transaction-live-chain-status (PRD-P2P-ITX-001) [test_tier_required]: chain/network identity is live-runtime only, with no launcher fallback. Trace: #2735 (task_9d8c13c232d04249b8c3e4dfb709acb2)
- [x] ideal-transaction-fee-quote-metadata (PRD-P2P-ITX-002) [test_tier_required]: payload hash, fee quote and client-request metadata are propagated without fee execution. Trace: #2735 (task_9d8c13c232d04249b8c3e4dfb709acb2)
- [x] ideal-transaction-phase-two-scope (PRD-P2P-ITX-003) [test_tier_required]: payer, sponsor, priority and actual fee economics stay out of Phase 1. Trace: #2735 (task_9d8c13c232d04249b8c3e4dfb709acb2)

## 依赖

- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/prd.index.md`
- transfer runtime、node、chain-runtime 与 client-launcher 路径

## 状态

Phase 1 metadata-only signed transaction closure is complete. The current authority is this no-date triad, not the deleted dated triad.

## 后续边界

Any real fee economics, sponsored transactions, priority ordering, payer separation or consensus-level request dedupe requires a new Phase 2+ fee/auth execution task and professional authority. Do not reopen this completed Phase 1 topic by claiming those capabilities from metadata fields.

## 验证

Dependencies remain the P2P module PRD/project/index and transfer runtime, node, chain-runtime and client-launcher paths. The inherited focused verification is cargo check for chain/web/client launchers, `transfer_auth`, `transfer_submit`, documentation governance, and `git diff --check`.
