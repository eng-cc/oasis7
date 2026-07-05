# Gameplay WASM-backed Regional Infrastructure: micro_depot Project

审计轮次: 1

## 入口定位

- PRD: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`
- Design: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.design.md`
- Root baseline: `doc/game/prd.md` / `PRD-GAME-016`
- Current task truth: GitHub issue `eng-cc/oasis7#1957`, `task_34c10ce5bafc46fa9c943178285d5a0b`

本文件只维护 `micro_depot` topic 的执行状态、owner lane 与验证入口；目标态要求以同名 PRD/design 为准。

## 当前状态

| 项目 | 当前口径 |
| --- | --- |
| Topic status | `candidate_formalized` |
| Stage impact | 不改变 `internal_playable_alpha_late` |
| Claim envelope impact | 不改变 `limited playable technical preview` |
| Current execution | #1957 将 design-only `micro_depot` 补成 PRD/design/project triplet，并同步 root/gameplay/index routes |
| Release boundary | 本专题完整性只证明 traceability；release/public claim 仍需 QA/playtest/live/provider evidence |

## 当前开放任务

| Task | PRD | Owner | Status | Next step | Trace |
| --- | --- | --- | --- | --- | --- |
| micro-depot-topic-triplet-formalization | PRD-GAME-016 | `tpm` + `producer_system_designer` + `gameplay_designer` + `repository_health_engineer` | in progress | 完成 PRD/project 新增、design header 修正、root/gameplay/index route 同步与 doc-governance verification。 | GitHub issue `eng-cc/oasis7#1957`; `doc/engineering/workflow/source-of-truth.md#1.2.2.1` |

## 任务拆解

| Step | Owner | Output | Verification |
| --- | --- | --- | --- |
| 1. Role slice integration | `tpm` | 合流 producer/system、gameplay、repository-health 三条 bounded slice 结论 | GitHub issue `eng-cc/oasis7#1957` evidence comments |
| 2. Topic triplet formalization | `tpm` | 新增 PRD/project，修正 design header | `./scripts/doc-governance-check.sh`; triplet pairing loop |
| 3. Route synchronization | `tpm` | 更新 `doc/game/prd.md`, `doc/game/project.md`, `doc/game/prd.index.md`, `doc/game/README.md`, `doc/game/gameplay/README.md` | `rg -n "PRD-GAME-016|micro_depot|gameplay-wasm-backed-regional-infrastructure-micro-depot" ...` |
| 4. Pre-PR verification | `tpm` + review roles | 文档治理、whitespace、local role review 和 closeout evidence | `git diff --check`; required repo-owned review |

## Follow-Up Implementation Lanes

| Lane | Owner role | Required tier | Entry condition | Done signal |
| --- | --- | --- | --- | --- |
| `micro-depot-wasm-abi-host-adapter` | `wasm_platform_engineer` + `runtime_engineer` | `test_tier_required` | PRD-GAME-016 accepted for implementation | Same input snapshot + module hash produces byte-identical proposal hash; schema/hash mismatch rejects. |
| `micro-depot-runtime-state-events` | `runtime_engineer` | `test_tier_required` | ABI/adapter boundary ready | Install/service/reclaim replay produces identical state; duplicate, out-of-range, unsupported-resource and unpaid-upkeep blockers are structured. |
| `micro-depot-quote-pipeline` | `runtime_engineer` + `gameplay_designer` | `test_tier_required` | runtime state/events available | Repair/logistics quote preview shows before/after with capped depot contribution and remaining blocker. |
| `micro-depot-viewer-api-surface` | `viewer_engineer` + `agent_engineer` | `test_tier_required` | canonical DTO and receipt fields available | Viewer and pure API show same quote, blocker, receipt, module evidence and next useful action; agent action remains explainable. |
| `micro-depot-gameplay-smoke` | `qa_engineer` + `gameplay_designer` | `test_tier_required` + `test_tier_full` | visible service path available | One repair/logistics action becomes cheaper, faster or less risky because of depot, and player can identify remaining blocker. |

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

- 更新日期: 2026-07-05
- 当前状态: `in_progress`
- 当前阶段判断: unchanged, `internal_playable_alpha_late`
- 当前 claim envelope: unchanged, `limited playable technical preview`
- 当前阻断条件:
  - 若 triplet 配对、root/gameplay/index route 或 doc-governance 失败，不得进入 PR closeout。
  - 若后续实现切片改变 runtime/WASM/player-facing 行为，必须重新派发对应专业 owner slice。
  - 若任何文案被用于 release/public claim，必须补 QA/playtest/liveops evidence。

## Trace

- 2026-07-05 / `task_34c10ce5bafc46fa9c943178285d5a0b`: producer/system, gameplay and repository-health slices agreed that design-only `micro_depot` is the clearest bible-grade gap. This project file was created to make the topic a formal PRD/design/project triplet without bloating root docs.
