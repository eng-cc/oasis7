# oasis7 主流公链测试体系对标与缺口矩阵（设计文档）

- 对应需求文档: `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments
- 原始基准日期: `2026-03-24`

审计轮次: 2

> Authority boundary: this design preserves the original benchmark background
> and dated follow-up observations only. Current network-tier status and
> blockers live in `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`
> and its companion runbook. A dated controlled `ready_for_live_candidate`
> packet is bounded to resettable, non-mainnet testnet candidacy; it does not
> establish public launch, a currently operating public fleet, open validator
> onboarding, or mainnet readiness.
> Historical update (2026-07-03): the external verifier /
> light-client-lite operator path gained bounded sampled world-head verification.
> That historical closure did not establish full light-client security,
> multi-client consensus equivalence, current network readiness, or release
> maturity. The dated gap snapshot remains at
> `doc/p2p/blockchain/p2p-current-mainstream-public-chain-gap-benchmark-2026-07-03.md`.

## 设计目标
- 把“主流公链怎么测”从泛泛经验，收成 oasis7 可执行的对标矩阵。
- 明确当前已有测试层、关键缺口和 producer 下一步优先级。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [benchmark AC-5](mainstream-public-chain-testing-benchmark.prd.md#ac-p2p-bench-005) | 区分原始 benchmark 对 drill / ceremony 的判断与后续窄范围 default/live finality pass/block/restore evidence | [DES-BENCH-G1 scope](mainstream-public-chain-testing-benchmark.design.md#des-bench-g1-scope) | QA 负责测试证据边界；producer / runtime / custody owners 负责各自 closure | 不证明完整 custody、genesis ceremony、广泛 governance/failover 覆盖或当前 fleet 状态 |
| [benchmark AC-6](mainstream-public-chain-testing-benchmark.prd.md#ac-p2p-bench-006) | 保持带日期的六层历史矩阵，并将动态 network-tier status 指回 formal authority | [DES-BENCH-L5 boundary](mainstream-public-chain-testing-benchmark.design.md#des-bench-l5-boundary) | QA 维护历史对标；network-tier owners 维护当前状态 | 不把 dated candidate packet 升级成 public launch 或 mainnet claim |

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
| Layer | 基准窗口 / 后续 scoped evidence | 证据 / authority | 结论 |
| --- | --- | --- | --- |
| `L0 spec/reference` | 已具备 | `./scripts/doc-governance-check.sh`、专题 PRD/project/design、registry import/audit 工具 | `present` |
| `L1 deterministic core` | 已具备，但缺 fuzz/property gate | `./scripts/ci-tests.sh required/full`、`main-token-regression.sh`、crate tests | `present_with_gap` |
| `L2 distributed system` | 已具备基础，但以库测和长跑为主 | `S4`、`S9`、`S10`、node/net/consensus/distfs tests | `present_with_gap` |
| `L3 user-facing closure` | 已具备 | `S6` Web-first UI 闭环、producer playtest 手册 | `present` |
| `L4 longrun/chaos/drill` | 长跑具备；后续窄范围 default/live `governance.finality.v1` drill 已记录 `2-of-3` pass、negative block 与 baseline restore，但不覆盖完整治理、custody 或 ceremony | `S9/S10`；`doc/testing/evidence/governance-registry-live-world-drill-finality-2026-03-24.md` 及下方历史补充 evidence | `partial` |
| `L5 network/release train` | legacy `shared_devnet` 仅作历史 rehearsal provenance。正式网络 readiness 由 network-tier authority 的独立 gate 决定；2026-07-06 记录的 11-lane packet 只支持 controlled/resettable/non-mainnet `ready_for_live_candidate`，不表示公开 launch、当前 live fleet 或 mainnet。每次当前状态须重新查正式 authority | `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md`；2026-07-06 dated packet | `historical_evidence_only` |

## 后续窄范围 governance evidence（证据记录：2026-03-24；本次复核：2026-09-26）

<a id="des-bench-g1-scope"></a>

以下记录作为原始 benchmark 快照的 scoped follow-up evidence 单独列出（source records dated 2026-03-24；本次复核 dated 2026-09-26）。它们修正“首轮真实 finality drill 仍未执行”的旧描述；范围仍限于 `governance.finality.v1` 的列明样本，不证明当前 fleet 状态、完整 signer custody、genesis ceremony、广泛 slot/failover 覆盖或 release readiness。

| 样本 | 观察结果 | 边界 |
| --- | --- | --- |
| [`governance-registry-live-world-drill-finality-2026-03-24.md`](../evidence/governance-registry-live-world-drill-finality-2026-03-24.md) | `signer03 -> signer04` rotation 保持 `2-of-3` 并通过；negative `2-of-2` audit 返回 `failover_blocked`；restore audit 回到 `ready_for_ops_drill`。 | 单一 finality rotation、threshold block 和 baseline restore 语义。 |
| [`governance-registry-live-world-drill-finality-multi-loss-rejoin-2026-03-24.md`](../evidence/governance-registry-live-world-drill-finality-multi-loss-rejoin-2026-03-24.md) | `signer01 + signer02` dual-loss 在 import policy 阶段被拒绝；baseline restore audit 通过。 | 不覆盖仍可写入 registry 的复杂 partial failover 或非-baseline rejoin 变体。 |
| [`governance-registry-live-world-drill-finality-baseline-rejoin-signer02-2026-03-24.md`](../evidence/governance-registry-live-world-drill-finality-baseline-rejoin-signer02-2026-03-24.md) | temporary offline 的 `signer02` 在 `2-of-2` degraded state 被 block，baseline rejoin 与 restore audit 回到 `ready_for_ops_drill`。 | 单一 signer temporary-offline / baseline-rejoin 样本。 |

因此，L4 不再把“首轮 default/live finality drill 没有 evidence”作为 blocker；L4 仍为 `partial`，因为这些记录没有关闭更广治理、custody、ceremony 和 long-run / release-train 证据。

## 原始缺口经后续证据收窄后的剩余范围
| Gap ID | 缺口 | 严重度 | owner | 下一步 |
| --- | --- | --- | --- | --- |
| `BENCH-G1` | 更广的 governance / negative-drill QA 范围仍不完整；窄范围 default/live finality pass/block/restore 已有记录，但 signer custody、genesis ceremony、更多 slot 与 failover 变体未由这些样本关闭 | `high` | `qa_engineer` + `runtime_engineer` | 扩充代表性 failure/restore 样本，并由对应 custody、ceremony 与 QA authority 验证；不得把已完成的 finality 样本写成整体 G1 完成 |
| `BENCH-G2` | fuzz/property-based gate 缺失 | `medium` | `runtime_engineer` + `qa_engineer` | 先定义最小 fuzz/property 切入面，再决定工具 |
| `BENCH-G3` | formal `public_testnet` candidate 与 `mainnet` readiness 使用独立 network-tier gates；legacy `shared_devnet` pass 不可替代目标网络状态。受控 candidate packet 不等于 public launch 或 mainnet readiness | `high` | `producer_system_designer` + `liveops_community` + `runtime_engineer` | `public_testnet` 当前状态只读 formal network-tier runbook 与当轮 required-lane packet；mainnet 另走 exit review、`MAINNET-1~4` 与 frozen-genesis / ceremony gates |
| `BENCH-G4` | 多客户端公链的“独立实现差分测试”在 oasis7 无等价替代 gate | `medium` | `runtime_engineer` | 评估独立 replay/verifier 或只读审计器路径 |

<a id="des-bench-l5-boundary"></a>

## Producer 结论
1. 原始快照已经不是“完全没有测试体系”，而是有一套偏 preview-to-hardening 的测试骨架；后续 scoped finality evidence 又补上了有限的真实 pass/block/restore 样本。
2. 这些证据没有关闭完整 `L4` 治理/custody/ceremony 范围，`L5` 当前状态也不由 benchmark 维护；成熟度判断不能外推成 live fleet 或 release 结论。
3. 因此下一步优先级应是：
   - `扩充 governance drill evidence，并补齐 custody / ceremony 范围`
   - `negative/fault drill`
   - `network rehearsal / release-train readiness`
   - `fuzz/property gate`

## 测试成熟度表述边界
- 以下只示范本 benchmark 支持或否定的测试成熟度表述；当前 release/network claims 仍须核对产品 claims authority 与 formal network-tier runbook。
- 本 benchmark 支持的成熟度表述：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `testing foundations exist, but mainstream public-chain-grade testing maturity is not yet complete`
- 本 benchmark 不支持的成熟度表述：
  - `mainstream public-chain-grade testing`
  - `mainnet-grade testing maturity`
  - `production release train is established`

## 11 验证映射

### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [benchmark AC-5](mainstream-public-chain-testing-benchmark.prd.md#ac-p2p-bench-005) | [DES-BENCH-G1 scope](mainstream-public-chain-testing-benchmark.design.md#des-bench-g1-scope) | 对照三份 2026-03-24 default/live evidence，确认文档不再称首轮 finality drill 未执行，且仍保留 custody / ceremony / broad-coverage gaps | [testing-manual.md](../../../testing-manual.md)：S0 文档治理入口；对照本设计所列 evidence paths 与目标 wording | 本任务的 `doc-governance-check.sh` 输出、两个 benchmark SHA-256 inventory rows 与 linked drill summaries | 静态文档检查不重放历史 live drill，也不证明当前 fleet、custody、ceremony 或 release readiness |
| [benchmark AC-6](mainstream-public-chain-testing-benchmark.prd.md#ac-p2p-bench-006) | [DES-BENCH-L5 boundary](mainstream-public-chain-testing-benchmark.design.md#des-bench-l5-boundary) | 确认六层矩阵继续作为 dated maturity snapshot，network status 指回 formal tier authority | [testing-manual.md](../../../testing-manual.md)：S0 文档治理入口；核对正式 network-tier runbook 对 candidate/public-launch/mainnet 的边界 | 本任务的 `doc-governance-check.sh` 输出与 linked 2026-07-06 runbook packet summary | 文档引用不构成当前网络 readiness 或 public fleet 证据 |
