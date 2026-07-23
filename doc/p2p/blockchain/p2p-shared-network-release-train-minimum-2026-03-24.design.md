# oasis7 legacy shared-network rehearsal / release-train background（设计文档）

- 对应需求文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md`
- 对应运行手册: `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`

审计轮次: 7

> Current canonical source: this design is legacy/rehearsal background for the old
> shared release-train model. Current network-tier truth lives in
> `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md`.
> Do not read `shared_devnet` pass as `public_testnet`, `mainnet`, public launch,
> or public large-world readiness.

## 设计目标
- 把 benchmark 中 `L5 network rehearsal / release-train readiness` 的缺口落成正式执行模型，而不是继续停留在口头 backlog。
- 明确 oasis7 下一阶段的最小 legacy rehearsal track、promotion 规则、rollback 规则与 claims gate。

## 当前结论
| 维度 | 当前状态 | 结论 |
| --- | --- | --- |
| local required/full + S6 + S9/S10 | 已具备 | `present` |
| governance drill clone/live evidence | 已具备基础 | `present_with_limited_coverage` |
| shared_devnet/staging/canary | 作为 legacy/rehearsal evidence，`shared_devnet` 已有 2026-05-24 `pass / eligible_for_promotion` 追溯结论；`staging/canary` 仍未正式执行，且该轨不再是目标 test 环境 | `legacy_rehearsal_pass` |
| public claims | 仍受 preview policy 限制 | `limited playable technical preview` + `crypto-hardened preview` |

## 三层 legacy rehearsal track
| Track | 目标 | 最小入口 | 最小通过标准 | 不算完成的情况 |
| --- | --- | --- | --- | --- |
| `shared_devnet` | 首次把统一 candidate 放到多人 network rehearsal 环境中运行 | 本地 gate 通过、candidate bundle 完整、共享访问路径明确 | 能被共享访问、版本固定、QA 有 `pass/block` 结论、可回滚到前一 bundle | 仍是单机私有 execution instance、只有运行命令没有结论 |
| `staging` | 做升级窗口、恢复、回滚和彩排 | `shared_devnet=pass`、升级窗口与 owner 值班明确 | promotion/rollback 各至少一轮，证据完整，liveops 认可 | 只是复用 shared_devnet、没有独立升级/恢复演练 |
| `canary` | 小范围真实发布轨道，验证 freeze/incident 响应 | `staging=pass`、duration/freeze 条件/incident owner 明确 | 有固定观察窗、可执行 freeze/rollback、incident 结论闭环 | 没有观察窗、没有 incident 结论、没有 fallback bundle |

## Candidate Bundle
| 字段 | 说明 |
| --- | --- |
| `candidate_id` | 本次 release candidate 唯一标识 |
| `git_commit` | 对应仓库提交 |
| `runtime_build` | 运行时/构建产物标识 |
| `world_snapshot_ref` | world 真值引用 |
| `governance_manifest_ref` | governance 真值引用 |
| `evidence_refs` | 本地 gate、drill、QA 文档引用 |

## 当前实现入口（RTMIN-1）
- 候选真值生成:
  - `./scripts/release-candidate-bundle.sh create`
- 候选真值校验:
  - `./scripts/release-candidate-bundle.sh validate`
- 最小 smoke:
  - `./scripts/release-candidate-bundle-smoke.sh`
- release gate 接线:
  - `./scripts/release-gate.sh --candidate-bundle <bundle.json>`
- 当前设计含义:
  - `release_candidate_bundle` 现在已具备机器可读 JSON 工件、路径哈希与 `git_commit` pinning。
  - `release-gate` 已可在进入 rehearsal track 前先校验 bundle 存在性、引用路径与 hash 漂移。
  - 本段记录 RTMIN-1 当时的实现入口；当前网络层 verdict 不在本文维护，见 formal network-tier project。

## 当前实现入口（RTMIN-2）
- QA gate 生成:
  - `./scripts/shared-network-track-gate.sh`
- QA gate smoke:
  - `./scripts/shared-network-track-gate-smoke.sh`
- QA 模板:
  - `doc/testing/templates/shared-network-track-gate-template.md`
  - `doc/testing/templates/shared-network-track-gate-lanes.shared_devnet.template.tsv`
  - `doc/testing/templates/shared-network-track-gate-lanes.staging.template.tsv`
  - `doc/testing/templates/shared-network-track-gate-lanes.canary.template.tsv`

## 当前实现入口（RTMIN-3）
- LiveOps runbook:
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
- LiveOps 模板:
  - `doc/testing/templates/shared-network-promotion-record-template.md`
  - `doc/testing/templates/shared-network-incident-template.md`
  - `doc/testing/templates/shared-network-incident-review-template.md`
  - `doc/testing/templates/shared-network-exit-decision-template.md`

## 当前实现入口（RTMIN-4）
- first `shared_devnet` dry run evidence:
  - `doc/testing/evidence/shared-network-shared-devnet-dry-run-2026-03-24.md`
- promotion record:
  - `doc/testing/evidence/shared-network-shared-devnet-promotion-record-2026-03-24.md`
- incident / hold record:
  - `doc/testing/evidence/shared-network-shared-devnet-incident-2026-03-24.md`
- actual gate outputs:
  - `output/release-candidates/shared-devnet-dry-run-20260324-01.json`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/release-gate/20260324-150030/release-gate-summary.md`
  - `output/shared-network/shared-devnet-dry-run-20260324-01/gate/shared_devnet-20260324-150230/summary.md`

## 当前实现入口（RTMIN-4A / RTMIN-5 前置编排）
- shared-devnet rehearsal orchestration:
  - `./scripts/shared-devnet-rehearsal.sh`
- orchestration smoke:
  - `./scripts/shared-devnet-rehearsal-smoke.sh`
- 当前设计含义:
  - 同一条命令现在可以围绕一个 `candidate_id` 串起 `release-candidate-bundle create/validate`、可选 `release-gate --dry-run`、same-candidate `headed Web + no-ui + pure_api` 复跑或证据复用、lane scaffold、`lanes.shared_devnet.tsv` 和 `shared-network-track-gate` 输出。
  - 它默认仍对 `shared_access`、`mixed_topology_baseline`、`governance_live_drill`、`short_window_longrun`、`rollback_target_ready` 维持保守语义；编排入口本身不等于 network-rehearsal `pass` 或 claims 升级。

## Track QA Required Lanes
| Track | Required lanes | Gate 结论规则 |
| --- | --- | --- |
| `shared_devnet` | `candidate_bundle_integrity` / `shared_access` / `multi_entry_closure` / `mixed_topology_baseline` / `governance_live_drill` / `short_window_longrun` / `rollback_target_ready` | 缺任一 required lane 直接 `block`；全部 `pass` 才可 promotion |
| `staging` | `candidate_bundle_integrity` / `shared_access` / `unified_candidate_gate` / `mixed_topology_rehearsal` / `governance_live_drill` / `upgrade_rehearsal` / `rollback_rehearsal` / `incident_template` | 任一 `block` 或缺 lane 即 `block`；存在 `partial` 则整体 `partial` |
| `canary` | `candidate_bundle_integrity` / `promotion_record` / `canary_window` / `mixed_topology_claim_review` / `rollback_rehearsal` / `incident_review` / `exit_decision` | 只有全部 `pass` 才能给出 `eligible_for_promotion`，否则维持 `hold_promotion` |

## QA Gate 规则
1. QA gate 只接受 `pass / partial / block` 三种 lane 状态。
2. 缺任一 required lane 时，整体 gate 直接输出 `block`。
3. required lanes 齐全但至少存在一条 `partial` 时，整体结论为 `partial`。
4. 只有 required lanes 齐全且所有 lanes 都是 `pass` 时，整体结论才为 `pass`，并给出 `eligible_for_promotion`。
5. QA gate summary 必须同时生成 `summary.json` 与 `summary.md`，不得只留口头结论。

## Promotion 规则
1. 任何 candidate 必须先完成本地 gate，再进入 `shared_devnet`。
2. 只有上一轨道结论为 `pass`，才允许 promotion 到下一轨道。
3. 一旦发现 commit/world/governance 真值漂移，立即 `freeze` 并退回重新编号。
4. `staging/canary` 的 `rollback` 目标必须是最近一次通过的 candidate bundle；首条 `shared_devnet pass` 若尚无历史 `pass` candidate，可接受受审计的 `bootstrap_restore_ready` fallback，但必须固定 restore steps、fallback owner ref 与 restoration scope。
5. `mixed_topology_baseline` 若要从 `partial` 升到 `pass`，必须满足三项同时成立：
   - same-window shared mixed-topology evidence 已固定并与当前 `candidate_id` 对账；
   - producer/QA 联审通过的 `pass_uplift_decision_ref` 已固定；
   - gate/evidence 输出中把 proxy drill 明确标注为近似，而不是 dedicated sentry/NAT lab 真值。
6. `shared_access` 若要从 `partial` 升到 `pass`，必须满足三项同时成立：
   - shared endpoint 已固定，且不再是单 owner 私有入口；
   - independent operator handoff / duty ref 已固定；
   - 独立 access evidence 已固定，并与当前 `candidate_id` 对账。
7. `rollback_target_ready` 若要从 `partial` 升到 `pass`，必须满足五项同时成立：
   - fallback bundle 已固定；
   - fallback gate ref 已固定；
   - fallback owner ref 已固定；
   - restore step ref 已固定；
   - restoration scope 已固定，并与当前 `candidate_id` 对账。

## LiveOps Window 规则
1. 每个 track 必须冻结唯一 `window_id`、`candidate_id`、`fallback_candidate_id`、`fallback_class`、`owners_on_duty` 和 `claim_envelope`。
2. `shared_devnet` 必须先留下 `promotion_record`，再开共享访问窗口。
3. `staging` 必须有独立的 upgrade window、incident template 和 rollback rehearsal 记录。
4. `canary` 必须有固定观察窗、`incident_review` 和 `exit_decision`，否则只能维持 `hold`。
5. 若共享访问失效、owner 值班断档或 public claims 越过 preview 边界，窗口立即转为 `frozen`。

## RTMIN-4 历史结论收束
1. 早期 RTMIN-4 的 `partial` / `hold_promotion` 结论已被后续 project/runbook/evidence 更新；不要继续把本节旧缺口当成当前 shared-devnet 状态。
2. 当前可读口径是：`shared_devnet` 已有 legacy rehearsal `pass / eligible_for_promotion` 追溯结论，但该结论只作为历史 rehearsal evidence，不等于 formal `public_testnet`、`mainnet`、public launch 或公开大世界 readiness。
3. `staging/canary` 仍未作为当前目标环境推进；formal test 环境真值转到 `public_testnet` six-lane readiness 与 formal network-tier docs。

## Partial / Block 语义
| 状态 | 含义 |
| --- | --- |
| `pass` | 轨道目标与证据完整，允许推进 |
| `partial` | 有环境或运行，但仍缺 shared access、mixed-topology、rollback，或 rollback 还只停留在未审计 bootstrap/provisional 级别 |
| `block` | 未满足 promotion 必需条件，不允许推进 |
| `frozen` | 事故或漂移导致临时冻结推进 |
| `restored` | 已回滚到上一通过 bundle，并留下恢复证据 |

## 对外口径控制
- 当前允许：
  - `limited playable technical preview`
  - `crypto-hardened preview`
  - `network rehearsal / release train is specified but not yet executed`
- 当前禁止：
  - `production release train is established`
  - `network rehearsal validated`
  - `mainnet-grade testing maturity`
