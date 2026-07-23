# oasis7 主流公链测试体系对标与缺口矩阵（项目管理文档）

- 对应设计文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.design.md`
- 对应需求文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md`
- 原始基准日期: `2026-03-24`

审计轮次: 2

> Authority boundary: this project file preserves benchmark/background history
> only. It does not own current network-tier truth or release maturity. Use
> `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md`
> for current `public_testnet` / `mainnet` readiness.

## 任务拆解（含 PRD-ID 映射）
- [x] BENCH-0 (PRD-P2P-BENCH-001/002/003/004) [test_tier_required]: 新建 benchmark 专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪。
- [x] BENCH-1 (PRD-P2P-BENCH-001/002) [test_tier_required]: 冻结主流公链测试分层模型与 oasis7 等价要求。
- [x] BENCH-2 (PRD-P2P-BENCH-002/003) [test_tier_required]: 映射 oasis7 当前 suites/evidence 到 benchmark layers，形成 gap matrix。
- [x] BENCH-3 (PRD-P2P-BENCH-003/004) [test_tier_required]: 冻结 producer 下一步优先级与 public claims 边界。

## 历史结论快照
- 2026-03-24 原始阶段口径:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - 总 verdict: `not_mainnet_grade`
- 后续历史回填后的 benchmark 结论:
  - `L0/L1/L3`: 已有正式基础
  - `L2`: 已有基础，但仍偏库测/长跑，缺 network rehearsal 维度
  - `L4`: 长跑已有，controller slot 与 finality slot 的 clone-world / default-live 首轮 governance drill 已完成；finality 已补到两条独立 single-signer recovery 样本、一条 multi-signer loss import-policy reject 样本、一条 `2-of-2 -> 2-of-3` non-baseline rejoin 样本，以及一条 baseline rejoin 样本，但覆盖范围仍有限
  - `L5`: first `shared_devnet` dry run 最初为 `partial`；2026-05-24 legacy rehearsal 曾回填 `pass / eligible_for_promotion` 历史结论，但它只作 benchmark L5 / legacy rehearsal evidence，不能替代任何当前 formal `public_testnet`、`mainnet` 或 public large-world launch readiness

## 依赖
- `testing-manual.md`
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
- `README.md`
- `doc/product/player-entry-distribution/release-communications-and-public-claims.prd.md`

## 验收命令（本轮）
- `rg -n "network rehearsal|release train|fuzz/property|governance drill|mainstream public-chain|legacy shared_devnet|not_mainnet_grade" doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.design.md doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.project.md doc/p2p/prd.md doc/p2p/project.md testing-manual.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: completed
- 下一步: benchmark L5 的 `shared_devnet` rehearsal 已有 pass 追溯结论；后续不应继续把目标写成“把 shared_devnet 从 partial 提升到 pass”，而应转向 formal `public_testnet` six-lane readiness、`staging/canary` rehearsal、以及 mainnet gates。即使引用 `shared_devnet pass`，也不得升级“对标主流公链测试成熟度”或 public large-world 相关口径。
- 最近更新: 2026-06-16
