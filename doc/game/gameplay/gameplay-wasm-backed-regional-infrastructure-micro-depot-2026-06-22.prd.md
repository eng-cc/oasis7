# Gameplay WASM-backed Regional Infrastructure: micro_depot PRD

- 对应设计文档: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.project.md`

审计轮次: 1

## 目标

- 将 `micro_depot` 从 design-only 补充提升为正式 `PRD-GAME-016` topic truth。
- 定义玩家通过小型、可审计、带 upkeep 的区域设施改变一次 repair / logistics quote 的体验承诺。
- 明确 WASM proposal 与 runtime authority 的边界，防止自由建造、任意 WASM 上传或 global governance 权力漂移。

## 范围

- 覆盖 `micro_depot` 的玩家动词、区域专业化阶段边界、install/service/upkeep/reclaim loop、状态经济规则、失败恢复、feedback surfaces、balance risks 和验证矩阵。
- 覆盖 topic triplet 与 root/gameplay/index 路由同步。
- 不覆盖具体 Rust/WASM 实现、任意玩家上传 WASM 治理、数值调参最终值、release/public claim 放行或 closed beta 阶段升级。

## 接口 / 数据

- PRD 主入口: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`
- 设计细节: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.design.md`
- 项目执行: `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.project.md`
- 根 baseline: `doc/game/prd.md` / `PRD-GAME-016`
- 关键 runtime 数据: `RegionalInfrastructure`, `MicroDepotEvalInput`, `MicroDepotProposal`, `MicroDepotServiceApplied`
- 关键 player-facing 数据: install quote, upkeep state, service radius, before/after preview, blocker, receipt, module evidence

## 里程碑

- M0 (2026-07-05): topic PRD/design/project triplet formalized and rooted in `PRD-GAME-016` routes.
- M1: WASM ABI + host adapter proves deterministic proposal hash.
- M2: runtime state/events and quote pipeline support install/service/reclaim with structured blockers.
- M3: Viewer / pure API / agent surfaces expose quote, receipt, module evidence and next useful action.
- M4: QA smoke proves one repair/logistics action becomes cheaper, faster or less risky because of depot, while remaining blocker is visible.

## 风险

- 如果 root docs 吸收完整矩阵，会重新制造 root PRD bloat。
- 如果 `micro_depot` 被写成 first-10-minute action，会破坏 trust/capability gate sequencing。
- 如果 WASM 被赋予 canonical mutation authority，会破坏 determinism、audit 和 replay safety。
- 如果效果过强、可堆叠或绕过 upkeep，会变成唯一正确解或 free-building exploit。
- 如果 receipt/module evidence 不可读，玩家仍无法知道自己造成了什么变化。

## 1. Executive Summary

- Problem Statement: `micro_depot` 已在设计补充中定义了可编程区域设施的玩家循环、WASM-backed 规则边界与 smoke gate，但缺少正式 PRD / project triplet，容易被误读为未承诺候选或直接实现承诺。
- Proposed Solution: 新增 `PRD-GAME-016`，把 `micro_depot` 收口为第一个 bible-grade 可编程区域设施专题：玩家通过小型、可审计、带 upkeep 的区域设施，改变一次 repair / logistics 行动的 quote，并获得可追溯 receipt。
- Success Criteria:
  - SC-1: 玩家能在同一专题内读懂为什么部署 depot、花了什么、改变了哪一次 repair / logistics 行动、还有什么 blocker 没解决。
  - SC-2: 设计明确保持 `regional specialist / limited-scope regional influence` 阶段边界，不进入首 10 分钟新手循环。
  - SC-3: `micro_depot` 的 WASM 权限边界清楚区分 proposal 与 runtime authority：WASM proposes, runtime validates / applies / signs。
  - SC-4: topic PRD / design / project、根 PRD、gameplay README 与 `prd.index.md` 可互相定位，不把详细规则塞回 root PRD。
  - SC-5: 本专题不升级当前阶段或对外 claim envelope；当前仍为 `internal_playable_alpha_late` / `limited playable technical preview`。

## 2. User Experience & Functionality

- User Personas:
  - 区域小玩家 / regional specialist: 已完成 first capability，想通过局部设施降低重复 repair / logistics 摩擦，而不是立即投靠 major power。
  - 回流玩家: 看到区域 blocker 后，需要一个可解释、可恢复、可审计的下一步，而不是只看 raw log。
  - `gameplay_designer`: 需要把玩家动词、loop、收益、失败和 anti-abuse 边界写成可验证玩法规则。
  - `producer_system_designer`: 需要冻结系统承诺、stage boundary、资源经济和 release claim 边界。
  - `runtime_engineer` / `wasm_platform_engineer`: 需要明确 WASM 只给 proposal，canonical state / accounting / receipt 由 runtime 拥有。
  - `viewer_engineer` / `agent_engineer` / `qa_engineer`: 需要玩家可读 surface、agent action contract 与 smoke matrix。
- User Scenarios & Frequency:
  - 首次区域设施解锁: 玩家完成 repair / logistics 闭环并看到重复 blocker 后触发。
  - 区域阻塞恢复: supply missing、route blocked、repair 等待过长或 logistics quote 风险过高时重复发生。
  - 维护与回收: upkeep 未付、设施超出有效范围、收益不足或玩家转向新 specialization 时发生。
- User Stories:
  - PRD-GAME-016: As a regional specialist, I want to deploy a small auditable depot that changes one repair/logistics quote, so that I can create local leverage without gaining free building, arbitrary scripting, or global governance power.
  - PRD-GAME-016A: As a player, I want before/after quote and receipt evidence, so that I understand what the depot changed and what remains blocked.
  - PRD-GAME-016B: As an implementation owner, I want WASM proposal authority separated from runtime validation/application/signing, so that programmable infrastructure stays deterministic, auditable, and bounded.
- Critical User Flows:
  1. Flow-MD-001: `regional pressure card -> repair/logistics blocker explanation -> suggested intervention: deploy micro_depot`
  2. Flow-MD-002: `install quote + upkeep + service radius preview -> player confirms -> MicroDepotInstalled receipt`
  3. Flow-MD-003: `repair/logistics quote request -> micro_depot.wasm evaluates proposal -> runtime validates -> before/after preview`
  4. Flow-MD-004: `player executes service action -> receipt shows depot contribution -> remaining blocker / next regional option`
  5. Flow-MD-005: `upkeep missing / out of range / unsupported resource / duplicate facility -> structured blocker -> pay upkeep, move target, reclaim, or choose another action`
- Functional Specification Matrix:

| Surface | Required fields / signals | Player verb | State transition | Rule / computation boundary | Owner |
| --- | --- | --- | --- | --- | --- |
| Install quote | `install_cost`, `upkeep_per_epoch`, `service_radius_cm`, `supported_resource_kinds`, `expected_effect`, `remaining_blockers` | inspect quote, deploy | `suggested -> quoted -> installed / rejected` | Quote must come from runtime, not Viewer guesswork | `runtime_engineer` + `viewer_engineer` |
| Facility state | `facility_id`, `owner_claim_id`, `location_id`, `status`, `module_id`, `wasm_hash`, `last_receipt_id` | inspect depot, pay upkeep, reclaim | `active -> upkeep_grace -> suspended -> reclaimed` | One `owner_claim_id + location_id` active micro_depot; status must be replay-safe | `runtime_engineer` |
| WASM proposal | `proposal_hash`, `effect_delta`, `explanation_code`, `consumed_resource_classes` | service repair/logistics | `not_applicable / blocked / applicable` | WASM cannot mint resources, mutate ownership, bypass upkeep, or write canonical world state | `wasm_platform_engineer` + `runtime_engineer` |
| Service preview | before/after cost, risk, wait, route, repair/logistics outcome | compare before/after, confirm service | `previewed -> applied / blocked` | Runtime caps all deltas and validates resource mutation / permission before applying | `runtime_engineer` |
| Receipt | `accepted_intent_id`, `execution_status`, `blocker_type`, `world_change_summary`, `module_hash`, `proposal_hash` | inspect receipt, choose next action | `applied -> next_blocker_visible / resolved` | Receipt must make depot contribution and remaining blocker legible | `viewer_engineer` + `qa_engineer` |

- Acceptance Criteria:
  - AC-1: `micro_depot` is formally registered as `PRD-GAME-016` and reachable from root/gameplay/index routes.
  - AC-2: The topic explicitly states it is not first-10-minute onboarding, free building, arbitrary player WASM upload, UGC market, Minecraft-style block placement, global governance, or starter-funding subsidy.
  - AC-3: Player verbs include diagnose regional blocker, inspect quote, deploy, pay upkeep, service repair/logistics, inspect receipt, and reclaim.
  - AC-4: The core loop covers blocker -> quote -> deploy -> before/after preview -> service action -> receipt -> remaining blocker / next regional option.
  - AC-5: State/economy rules cover install cost, upkeep, service radius, one depot per claim/location, supported resources, effect caps, degradation/grace, and funding provenance.
  - AC-6: Failure/recovery covers unpaid upkeep, out of range, unsupported resource, missing regional blocker receipt, duplicate facility, insufficient funds, permission denial, and reclaim.
  - AC-7: Every applied service records module id/version/hash, schema version and proposal hash for replay/audit.
  - AC-8: QA smoke proves one repair/logistics action becomes measurably cheaper, faster, or less risky because of depot, and the player can identify remaining blocker.
- Non-Goals:
  - No block placement, digging, terraforming, free-build construction, or direct embodied control.
  - No arbitrary player-uploaded WASM in MVP; only repo-authored allowlisted module hashes.
  - No bypass of claim scope, upkeep, restricted starter fund transfer guards, or resource accounting.
  - No global governance power, alliance leadership, or closed-beta/release claim upgrade.

## 3. AI / Agent System Requirements

- Tool Requirements:
  - Agent action contract must surface `InstallMicroDepot`, `EvaluateMicroDepotQuote`, `ServiceRepairFromDepot`, `ServiceLogisticsFromDepot`, `PayDepotUpkeep`, `ReclaimDepot`, and `InspectDepot` as bounded, explainable actions.
  - Agent must not silently deploy a depot without player-visible quote, expected effect, upkeep, and remaining blocker.
- Evaluation Strategy:
  - Agent output is acceptable only when it preserves accepted intent, quote readability, receipt causality, and recovery suggestions.
  - If the agent chooses another action, it must explain why depot is out of range, unaffordable, unsupported, duplicate, suspended, or lower value than an alternative.

## 4. Technical Specifications

- Architecture Overview:
  - `micro_depot` is a WASM-backed regional facility. WASM evaluates a proposal against a runtime-provided input snapshot; runtime validates, caps, applies, signs and emits receipt.
  - Viewer and pure API render canonical DTOs only; they do not inspect WASM or infer benefit from events.
- Integration Points:
  - `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.design.md`
  - `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`
  - `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
  - `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
  - `doc/world-simulator/viewer/viewer-pixel-world-player-leverage-production-readability-2026-05-28.brainstorm.md`
- Edge Cases & Error Handling:
  - Missing `regional_blocker_receipt_id`: install is refused; player receives blocker explaining required prior repair/logistics receipt.
  - Duplicate `owner_claim_id + location_id`: install is refused with existing depot pointer and inspect/reclaim option.
  - Unpaid upkeep: service is blocked or degraded; player can pay upkeep, reclaim, or choose another intervention.
  - Out of range: proposal returns not applicable; UI must show distance/radius explanation and next valid target when known.
  - Unsupported resource: proposal is blocked; UI must name unsupported resource class without suggesting arbitrary resource minting.
  - Insufficient funds: quote remains visible but install/action cannot proceed; restricted starter funding cannot become infinite facility subsidy.
  - Unknown module hash or schema mismatch: runtime refuses proposal and records module evidence blocker.
- Non-Functional Requirements:
  - NFR-MD-1: Given identical input snapshot and module hash, proposal hash must be byte-identical.
  - NFR-MD-2: 100% of applied depot services must emit module hash, proposal hash, before/after quote, world change summary and receipt id.
  - NFR-MD-3: 100% of player-facing previews must include remaining blocker or next useful action.
  - NFR-MD-4: Any formal playability or release claim must cite QA/playtest evidence; this PRD alone is not release evidence.
- Security & Privacy:
  - WASM has no wall clock, network, filesystem, host escape, nondeterministic RNG, ownership mutation, resource minting or canonical state write authority.
  - Runtime preserves funding provenance and rejects restricted starter fund transfer or scope bypass.

## 5. Risks & Roadmap

- Phased Rollout:
  - R0: Formalize `PRD-GAME-016` triplet and routing without changing stage or claim envelope.
  - R1: Implement WASM ABI + host adapter with deterministic proposal hash.
  - R2: Add canonical state/events and replay-safe install/service/reclaim.
  - R3: Integrate repair/logistics quote pipeline and before/after preview.
  - R4: Add Viewer / pure API DTOs, receipt surface, module evidence, and blocker taxonomy.
  - R5: Run gameplay smoke and QA matrix before any release/public claim.
- Technical / Gameplay Risks:
  - If effect is too weak, depot feels like tax; if too strong, it becomes the only correct action.
  - If upkeep appears too early, starter funding and first capability flow become punishing.
  - If module governance opens arbitrary upload too early, MVP becomes a security platform problem.
  - If receipt fields are thin, Viewer falls back to guessing causality from recent events.
  - If future facilities expand too fast, oasis7 can be misread as direct construction gameplay rather than indirect-control civilization simulation.

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | Corresponding task / slice | Evidence tier | Validation method | Regression scope |
| --- | --- | --- | --- | --- |
| PRD-GAME-016 | `micro-depot-topic-triplet-formalization` | `test_tier_required` | doc governance, root/gameplay/index triplet reachability, no stage/claim drift | topic authority and design-bible completeness |
| PRD-GAME-016 | `micro-depot-wasm-abi-host-adapter` | `test_tier_required` | same input produces byte-identical proposal hash; schema/hash mismatch rejects | WASM determinism and adapter boundary |
| PRD-GAME-016 | `micro-depot-runtime-state-events` | `test_tier_required` | install/service/reclaim replay produces identical state and structured blockers | canonical facility state |
| PRD-GAME-016 | `micro-depot-quote-pipeline` | `test_tier_required` | repair/logistics preview shows before/after with capped depot contribution | player quote readability |
| PRD-GAME-016 | `micro-depot-viewer-api-surface` | `test_tier_required` | Viewer and pure API show same quote, blocker, receipt and module evidence | player-facing receipt and parity |
| PRD-GAME-016 | `micro-depot-gameplay-smoke` | `test_tier_required` + `test_tier_full` | one repair/logistics action becomes cheaper/faster/less risky and remaining blocker is visible | regional specialist leverage |

- Decision Log:

| Decision ID | Selected option | Rejected alternative | Evidence / rationale |
| --- | --- | --- | --- |
| DEC-MD-001 | Promote `micro_depot` to formal `PRD-GAME-016` topic triplet | Leave as design-only supplement while README routes to it | Design already defines loop, state, receipt and smoke gates; role slices flagged design-only status as the clearest bible-grade gap. |
| DEC-MD-002 | Keep `micro_depot` in regional specialist / limited-scope regional influence | Make it a first-10-minute onboarding action | It requires claim/upkeep/blocker/receipt literacy and would steal focus from trust/capability gates. |
| DEC-MD-003 | WASM proposes, runtime validates/applies/signs | Let WASM mutate canonical state or account balances | Determinism, replay, security and auditability require runtime authority. |
| DEC-MD-004 | MVP only allows repo-authored allowlisted module hash | Allow arbitrary player-uploaded WASM | Arbitrary upload would move the slice from gameplay facility to security/governance platform work. |
| DEC-MD-005 | Depot affects one bounded repair/logistics quote and receipt | Treat depot as global buff or governance power | Bounded quote contribution preserves small-player regional leverage without global power creep. |
