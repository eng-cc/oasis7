# oasis7 主链 mainnet-grade readiness 硬化路线（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`

审计轮次: 1
## 任务拆解（含 PRD-ID 映射）
- [x] MAINNET-0 (PRD-P2P-MAINNET-001/002/003/004) [test_tier_required]: 新建 mainnet-grade readiness 硬化路线专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪。
- [x] MAINNET-1 (PRD-P2P-MAINNET-001/002) [test_tier_required]: 由 `producer_system_designer` + `runtime_engineer` 输出“生产级 signer custody / keystore”专题，冻结生产 signer 源、rotation/revocation 与 audit trail 完成定义。
- [x] MAINNET-2 (PRD-P2P-MAINNET-002) [test_tier_required]: 由 `runtime_engineer` 输出“治理 finality signer 外部化”专题，去掉 deterministic local seed production path，并定义 failover/rotation/revocation。
- [x] MAINNET-3 (PRD-P2P-MAINNET-003) [test_tier_required + test_tier_full]: 由 `producer_system_designer` + `qa_engineer` 输出“创世 freeze/ceremony/QA gate”专题，冻结 recipient/controller/signer policy 真值并沉淀证据 bundle。
- [x] MAINNET-4 (PRD-P2P-MAINNET-001/004) [test_tier_required]: 由 `producer_system_designer` + `liveops_community` 执行最终 readiness 复评，更新 public claims policy 与阶段 verdict。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - 总 verdict: `not_mainnet_grade`
- 已收口:
  - `TransferMainToken/ClaimMainTokenVesting/InitializeMainTokenGenesis/DistributeMainTokenTreasury` 已进入 shared signed payload gating。
  - Web/native 转账 UI 已补本地签名接线与 Web-first QA 证据。
  - genesis/treasury 已有 controller slot binding 与本地 signer allowlist / threshold enforcement。
- 仍待完成:
  - 生产级 signer custody / keystore 的工程替换仍待后续 runtime 实施，当前只完成规格 gate。
  - 治理 signer 外部化的 runtime 工程替换仍待后续实现，当前只完成规格 gate；其中 validator / finality signer 的准入/激活流程已冻结目标态，但 candidate registry / activation action / probation runbook 仍未落地。
  - 创世 freeze/ceremony/QA 的真实地址、公钥、dry-run 与最终 QA `pass` 仍待后续执行，当前只完成规格 gate。

## 依赖
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md`
- `crates/oasis7/src/bin/oasis7_chain_runtime/node_keypair_config.rs`
- `crates/oasis7/src/runtime/world/governance.rs`
- `crates/oasis7_node/src/types.rs`
- `crates/oasis7_node/src/node_runtime_core.rs`
- `testing-manual.md`

## 验收命令（本轮）
- `rg -n "not_mainnet_grade|crypto-hardened preview|MAINNET-1|MAINNET-4|deterministic local seed|TBD_BEFORE_MINT|pending_binding" doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.design.md doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.project.md doc/p2p/prd.md doc/p2p/project.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 下一步: 若继续推进，应进入 execution workstreams，分别落地 signer custody、governance truth externalization、genesis binding/ceremony/QA；在这些项完成前，继续执行当前 preview claims policy。
- 最近更新: 2026-03-23
