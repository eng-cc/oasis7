# oasis7 主流公链测试体系对标与缺口矩阵（设计文档）

- 对应需求文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md`
- 对应项目管理文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.project.md`
- 原始基准日期: `2026-03-24`

审计轮次: 2

> Authority boundary: this design preserves benchmark background and historical
> observations only. Current network-tier status and blockers live in
> `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md`.
> Historical update (2026-07-03): the external verifier /
> light-client-lite operator path gained bounded sampled world-head verification.
> That historical closure did not establish full light-client security,
> multi-client consensus equivalence, current network readiness, or release
> maturity. The dated gap snapshot remains at
> `doc/p2p/blockchain/p2p-current-mainstream-public-chain-gap-benchmark-2026-07-03.md`.

## 设计目标
- 把“主流公链怎么测”从泛泛经验，收成 oasis7 可执行的对标矩阵。
- 明确当前已有测试层、关键缺口和 producer 下一步优先级。

## 主流公链测试分层模型
| Layer | 主流公链常见做法 | oasis7 等价要求 |
| --- | --- | --- |
| `L0 spec/reference` | 规范向量、格式/状态转换参考样例、静态门禁 | 文档治理、工件一致性、协议/治理 registry reference fixture |
| `L1 deterministic core` | unit/integration/property/状态机类测试 | `required/full`、runtime/simulator/viewer live/unit integration、后续补 fuzz/property gate |
| `L2 distributed system` | 多节点/网络/复制/升级兼容 | `S4` + `S9/S10` + world/consensus/distfs 多节点一致性 |
| `L3 user-facing closure` | 钱包/浏览器/节点运维路径真实可用 | `S6` Web-first UI 闭环、producer playtest、viewer auth 路径 |
| `L4 longrun/chaos/drill` | 长跑、故障注入、事故演练、key rotation/validator failover | `S9/S10` 长跑 + governance rotation/revocation/failover 实际 drill |
| `L5 network/release train` | devnet/testnet/canary、共享环境升级演练、发布列车 | 等价映射包括 formal `public_testnet` readiness、mainnet gates 和 legacy `shared_devnet` rehearsal evidence；具体当前状态回到各自专业权威 |

## oasis7 历史映射快照
| Layer | 当前状态 | 当前证据 | 结论 |
| --- | --- | --- | --- |
| `L0 spec/reference` | 已具备 | `./scripts/doc-governance-check.sh`、专题 PRD/project/design、registry import/audit 工具 | `present` |
| `L1 deterministic core` | 已具备，但缺 fuzz/property gate | `./scripts/ci-tests.sh required/full`、`main-token-regression.sh`、crate tests | `present_with_gap` |
| `L2 distributed system` | 已具备基础，但以库测和长跑为主 | `S4`、`S9`、`S10`、node/net/consensus/distfs tests | `present_with_gap` |
| `L3 user-facing closure` | 已具备 | `S6` Web-first UI 闭环、producer playtest 手册 | `present` |
| `L4 longrun/chaos/drill` | 长跑具备，治理真实 drill 证据缺失 | `S9/S10` 已有；governance import/audit/runbook 已有；真实 pass/block 证据未回写 | `partial` |
| `L5 network/release train` | legacy `shared_devnet` rehearsal 留有历史追溯；任何当前状态均不在本 benchmark 维护，转由 formal network-tier project 维护 | `historical_evidence_only` |

## 当前高优先级缺口
| Gap ID | 缺口 | 严重度 | owner | 下一步 |
| --- | --- | --- | --- | --- |
| `BENCH-G1` | 真实 governance drill / negative drill / QA evidence 未完成 | `high` | `qa_engineer` + `runtime_engineer` | 先做 clone-world 证据，再做 default/live execution world 正式留档 |
| `BENCH-G2` | fuzz/property-based gate 缺失 | `medium` | `runtime_engineer` + `qa_engineer` | 先定义最小 fuzz/property 切入面，再决定工具 |
| `BENCH-G3` | formal `public_testnet` / `mainnet` readiness 仍需独立 gate，legacy `shared_devnet` pass 不可替代目标网络状态 | `high` | `producer_system_designer` + `liveops_community` + `runtime_engineer` | 走 formal network-tier six-lane readiness、exit review 与 mainnet gates |
| `BENCH-G4` | 多客户端公链的“独立实现差分测试”在 oasis7 无等价替代 gate | `medium` | `runtime_engineer` | 评估独立 replay/verifier 或只读审计器路径 |

## Producer 结论
1. oasis7 当前已经不是“完全没有测试体系”，而是已经有一套偏 preview-to-hardening 的测试骨架。
2. 真正拉不开差距的不是再多写几个 unit test，而是把 `L4/L5` 补成持续执行、可审计、能升级的体系。
3. 因此下一步优先级应是：
   - `真实 governance drill 证据`
   - `negative/fault drill`
   - `network rehearsal / release-train readiness`
   - `fuzz/property gate`

## 对外口径影响
- 当前允许：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `testing foundations exist, but mainstream public-chain-grade testing maturity is not yet complete`
- 当前禁止：
  - `mainstream public-chain-grade testing`
  - `mainnet-grade testing maturity`
  - `production release train is established`
