# Gameplay WASM-backed Regional Infrastructure: micro_depot Project

审计轮次: 2

## 入口定位

- PRD: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`
- Design: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.design.md`
- Root baseline: `doc/game/prd.md` / `PRD-GAME-016`
- Current task truth: GitHub issue `eng-cc/oasis7#2279`, `task_511d22454fee4f62b893df3462242cc5`
- Current draft PR: GitHub pull request `eng-cc/oasis7#2289`

本文件只维护 `micro_depot` topic 的执行状态、owner lane 与验证入口；目标态要求以同名 PRD/design 为准。

## 当前状态

| 项目 | 当前口径 |
| --- | --- |
| Topic status | `measured_supply_implemented_pre_merge` |
| Stage impact | 不改变 `internal_playable_alpha_late` |
| Claim envelope impact | 不改变 `limited playable technical preview` |
| Current execution | #2279 / draft PR #2289 实现 measured-supply runtime/accounting、`micro_depot.eval.v2` 与精确 v1 compatibility；当前为 pre-merge verification，不表示已合入 `main` |
| Release boundary | 当前证据只支持 branch implementation / pre-merge verification；合入、release/public claim 仍需 canonical PR gate、merge receipt 与相应 QA/playtest/live/provider evidence |

## 当前开放任务

| Task | PRD | Owner | Status | Next step | Trace |
| --- | --- | --- | --- | --- | --- |
| micro-depot-measured-supply-accounting | PRD-GAME-016 | `producer_system_designer` + `runtime_engineer` + `wasm_platform_engineer` + `qa_engineer` | implemented on branch / pre-merge verification | 完成 draft PR #2289 的 current-head verification、repo-owned review 与 PR merge gate；合入前不得写成 mainline complete。 | GitHub issue `eng-cc/oasis7#2279`; draft PR `eng-cc/oasis7#2289`; `task_511d22454fee4f62b893df3462242cc5` |

## 任务拆解

| Step | Owner | Output | Verification |
| --- | --- | --- | --- |
| 1. Measured contract | `producer_system_designer` + `runtime_engineer` + `wasm_platform_engineer` | 将 single-commission consumable stock、内部 throughput ceiling、精确 debit、原子 apply 与 receipt before/after 绑定为 v2 normative contract | PRD/design acceptance and runtime/accounting tests in draft PR #2289 |
| 2. Runtime/accounting implementation | `runtime_engineer` | Data-only commissioning `10 = 2 sink + 8 transfer`、初始 throughput `16`、service 原子扣减、reclaim/reinstall 与 replay accounting | targeted micro-depot tests and `cargo check` evidence on GitHub issue #2279 |
| 3. Compatibility implementation | `wasm_platform_engineer` + `runtime_engineer` | 保留历史 v1 wire、canonical serialization、input/proposal hash projection；v2 measured fields 不进入 v1 projection | v1 golden compatibility evidence on GitHub issue #2279 |
| 4. Pre-merge integration | `tpm` + involved review roles | 冻结 current head，完成 fresh verification、review、required checks、comment/thread gate 与 merge receipt | draft PR #2289; canonical PR gate evidence |

## Current PR Implementation Lanes

| Lane | Owner role | Required tier | Entry condition | Done signal |
| --- | --- | --- | --- | --- |
| `micro-depot-wasm-abi-host-adapter` | `wasm_platform_engineer` + `runtime_engineer` | `test_tier_required` | implemented in draft PR #2289 | Same `micro_depot.eval.v2` input snapshot + module hash produces byte-identical measured proposal hash; schema/hash mismatch rejects; v1 golden preserves exact historical pre-change wire/action and input/proposal hash projection. |
| `micro-depot-runtime-state-events` | `runtime_engineer` | `test_tier_required` | implemented in draft PR #2289 | Data-only v2 install debits `10 Data` as `2` sink + `8` transfer; service consumes finite stock; at zero, service and upkeep reject without charge; reclaim refunds zero/destroys remainder; only full-cost reinstall creates fresh `8/16`; replay/blockers remain structured. |
| `micro-depot-measured-supply-accounting` | `runtime_engineer` + `wasm_platform_engineer` + `qa_engineer` | `test_tier_required` + `test_tier_full` | implemented in draft PR #2289 | Exact stock/throughput debit and receipt before/after reconcile; failed apply is atomic; exact historical v1 wire/hash compatibility remains covered. This status is branch-local until merge receipt exists. |

## Authorized Follow-Ups

| Follow-up | Owner role | Authorization boundary | Required evidence |
| --- | --- | --- | --- |
| Authorized refill | `gameplay_designer` + `runtime_engineer` | Define inventory source authority, refill costs, transfer/debit event and anti-abuse rules before adding any refill action. | Dedicated acceptance, runtime replay/accounting tests and player-facing blocker/recovery review. |
| Canonical epoch rollover/reset | `gameplay_designer` + `runtime_engineer` | Define authoritative epoch clock, rollover/reset trigger and event semantics; upkeep or arbitrary ticks must not refill throughput implicitly. | Deterministic clock/event contract, boundary/replay tests and structured recovery evidence. |
| Configurable commissioning and debit curves | `gameplay_designer` + `producer_system_designer` + `runtime_engineer` | Migrate versioned `10/8/16` facts and repair/logistics debit curves only after balance authority approves config and migration behavior. | Balance rationale, versioned config/migration contract, determinism and regression evidence. |

## Validation Entry

- Topic traceability:
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
  - `for f in doc/game/gameplay/*.prd.md; do base=${f%.prd.md}; [ -f "$base.design.md" ] || echo "missing design for $f"; [ -f "$base.project.md" ] || echo "missing project for $f"; done`
- Route checks:
  - `rg -n "PRD-GAME-016|micro_depot|gameplay-wasm-backed-regional-infrastructure-micro-depot" doc/game/prd.md doc/game/project.md doc/game/prd.index.md doc/game/README.md doc/game/gameplay/README.md doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.*.md`

## Blockers / Guardrails

- Do not route this topic into first-10-minute onboarding or free build UI.
- Do not expose arbitrary player-uploaded WASM before security/governance owner lanes exist.
- Do not let depot effects bypass claim scope, upkeep, resource accounting or restricted starter funding provenance.
- Do not treat upkeep as service inventory, invent refill/rollover authority, reserve stock during preview, accept class-only consumption as the v2 norm, or allow partial stock/throughput debit.
- Current v2 MVP is a single-commission consumable depot and accepts exactly canonical `["data"]`; empty/non-data/mixed/duplicate/case variants reject. Install debits `10 Data = 2` non-refundable sink + `8` commissioned transfer. Stock depletion ends service life; upkeep never replenishes and must reject/no charge at zero. Recovery is only destructive reclaim plus a fresh full-cost `10/8/16` install with no carry. Throughput `16` is an internal defense-in-depth ceiling/replay invariant, not a separately reachable gameplay blocker or epoch loop. Authorized refill, canonical epoch rollover/reset, and configurable commissioning/debit-curve balance remain future work.
- Do not use this PRD/project to claim closed beta, production readiness, or live release.
- If future edits change stage, gate verdict, preview cadence or public claim wording, route through `producer_system_designer`, `qa_engineer`, and `liveops_community`.

## 依赖

- Workflow authority: `doc/engineering/workflow/source-of-truth.md#1.2.2.1`
- Root gameplay baseline: `doc/game/prd.md`
- Gameplay routing: `doc/game/gameplay/README.md`, `doc/game/prd.index.md`
- Related topics:
  - `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`
  - `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
  - `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`

## 状态

- 更新日期: 2026-07-15
- 当前状态: `implemented_on_branch_pre_merge_verification`
- 当前阶段判断: unchanged, `internal_playable_alpha_late`
- 当前 claim envelope: unchanged, `limited playable technical preview`
- 当前阻断条件:
  - 若 draft PR #2289 current-head verification、required checks、review comments/threads 或 mergeability gate 未通过，不得宣称已合入或完成。
  - 若 authorized follow-up 改变 refill、epoch、config 或 balance 行为，必须新建 task truth 并重新派发对应专业 owner slice。
  - 若任何文案被用于 release/public claim，必须补 QA/playtest/liveops evidence。

## Trace

- 2026-07-05 / `task_34c10ce5bafc46fa9c943178285d5a0b`: producer/system, gameplay and repository-health slices agreed that design-only `micro_depot` is the clearest bible-grade gap. This project file was created to make the topic a formal PRD/design/project triplet without bloating root docs.
- 2026-07-15 / `task_511d22454fee4f62b893df3462242cc5` / GitHub #2279 / draft PR #2289: measured-supply runtime/accounting and exact v1 compatibility are implemented on the task branch and are in pre-merge verification. This supersedes #1957 as current execution truth; #1957 remains historical topic-formalization evidence only.
