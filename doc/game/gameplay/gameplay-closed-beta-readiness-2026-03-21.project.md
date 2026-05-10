# Gameplay 封闭 Beta 准入门禁（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`

审计轮次: 4

## 任务拆解

- [x] TASK-GAMEPLAY-CB-001 (`PRD-GAME-009`) [test_tier_required]: 冻结“当前处于 internal_playable_alpha_late、下一阶段目标为 closed_beta_candidate”的正式口径，并完成 `game` 根 PRD / project、`gameplay-top-level-design` 主文档、索引、handoff 与 devlog 挂载。
- [x] TASK-GAMEPLAY-CB-002 (`PRD-GAME-009`) [test_tier_required + test_tier_full]: `runtime_engineer` 已收口 five-node no-LLM soak、replay/rollback drill 与 longrun release gate 的候选版本证据，证明当前技术底座已达到封闭 Beta 准入下限。
- [x] TASK-GAMEPLAY-CB-003 (`PRD-GAME-009`) [test_tier_required]: `viewer_engineer` 已完成 `PostOnboarding` 首屏降噪收口、同候选 headed Web/UI rerun 与 playability 卡片回写；当前主目标、进度和下一步建议已成为封闭 Beta 候选级首屏主焦点。
- [x] TASK-GAMEPLAY-CB-004 (`PRD-GAME-009`) [test_tier_required + test_tier_full]: `qa_engineer` 已建立统一 `closed_beta_candidate` release gate，汇总 headed Web/UI、pure API、no-UI smoke、longrun/recovery 与趋势基线，并在最近 7 天窗口 trend baseline 刷新后将当前正式建议更新为 `pass`。
- [x] TASK-GAMEPLAY-CB-005 (`PRD-GAME-009`) [test_tier_required]: `liveops_community` 收口封闭 Beta 候选招募/反馈/事故回流 runbook 与对外禁语清单，在 producer 放行前保持 `technical preview` 口径。
- [x] TASK-GAMEPLAY-CB-006 (`PRD-GAME-009`) [test_tier_required]: `producer_system_designer` 已基于 `TASK-GAMEPLAY-CB-002/003/004/005` 的统一证据完成阶段评审，并决定当前继续保持 `internal_playable_alpha_late`；理由是虽然 unified gate 已 `pass`，但当前 claim envelope 与 liveops 节奏仍维持 `limited playable technical preview`，暂不切到 `closed_beta_candidate` 口径。
- [x] TASK-GAMEPLAY-CB-007 (`PRD-GAME-009`) [test_tier_required]: 基于最新制作人口径决策，将当前对外 claim envelope 从 `technical preview / not playable yet` 收口为 `limited playable technical preview`，并同步回写 game/readme/liveops 当前真值文档。

## 依赖

- `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-03-22
- 当前状态: completed
- 当前 owner: `producer_system_designer`
- 下一任务: `待新需求（继续监控 unified gate / trend baseline / liveops claim envelope）`
- 已完成补充:
  - `TASK-GAMEPLAY-CB-007` 已完成当前有效口径收口：阶段继续保持 `internal_playable_alpha_late`，但对外 claim envelope 从 `technical preview / not playable yet` 更新为 `limited playable technical preview`；`closed beta / play now / live now` 仍为禁语。
- 已完成补充:
  - `TASK-GAMEPLAY-CB-002` 已交付 `doc/testing/evidence/closed-beta-runtime-s10-2026-03-22.md`，其中包含 clean-room `120s` / `600s+` soak 通过样本与 replay/rollback required-tier drill 结果，runtime lane 已可交 QA 并入 unified gate。
  - `TASK-GAMEPLAY-CB-003` 已交付 `doc/playability_test_result/card_2026_03_22_15_56_13.md` 与 `output/playwright/playability/closed-beta-20260322/post-onboarding-20260322-155613/`：同候选 headed Web/UI rerun 已 `pass`，且 `AgentNotFound` 历史噪音不再占据右侧 chatter 焦点。
  - `TASK-GAMEPLAY-CB-004` 已补齐同候选 pure API / no-UI rerun：`output/playwright/playability/pure-api-required-20260322-183650/`、`output/playwright/playability/pure-api-full-20260322-183750/` 与 `output/playwright/playability/post-onboarding-headless-20260322-183832/` 均已 `pass`，统一 gate 的剩余 blocker 已收敛到 trend baseline。
  - `TASK-GAMEPLAY-CB-004` 已进一步刷新最近 7 天 trend baseline 到 `first-pass=100% / escape=0% / fix-time=0d`，统一 QA gate 现已转为 `pass`，并可交 `producer_system_designer` 做阶段评审。
  - `TASK-GAMEPLAY-CB-005` 已交付 `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.{prd,design,project}.md`、`doc/playability_test_result/templates/closed-beta-candidate-{feedback-log-guide,incident-templates}-2026-03-22.md`，并把 `technical preview` 口径边界回写到 `readme` 模块正式追踪。
- 阻断条件:
  - 若统一 release gate 尚未建立，则不得使用 `closed beta` 口径。
  - 若 trend baseline 未达标，则不得把当前阶段从 `internal_playable_alpha_late` 提升到 `closed_beta_candidate`。
  - 若 liveops 对外 runbook 仍缺招募/反馈/事故回流模板，则不得扩大对外承诺。
- 说明:
  - 本专题的目标是“阶段准入治理”，不是新增玩法功能。
  - 允许保持当前阶段，只要阻断项被如实记录并重新拆回 owner。
  - `TASK-GAMEPLAY-CB-002` 当前最新事实：clean-room `600s+` soak `output/longrun/closed-beta-candidate-20260322/20260322-121320` 已 `process_status=ok / metric_gate=pass`，且 replay/rollback required-tier drill 两条测试均通过，因此 runtime lane 已完成并可交 QA。
  - `TASK-GAMEPLAY-CB-003` 当前最新事实：fresh bundle headed Web/UI rerun `output/playwright/playability/closed-beta-20260322/post-onboarding-20260322-155613` 已 `pass`，并通过人工复核确认 `PostOnboarding` 主目标仍为首屏焦点。
  - `TASK-GAMEPLAY-CB-004` 当前最新事实：统一 QA gate 文档已建立且 runtime / headed Web/UI / pure API / no-UI / trend baseline lane 均已转为 `pass`；当前专题已无 QA 技术 blocker，下一步只剩 `TASK-GAMEPLAY-CB-006` 的 producer 阶段评审。
  - `TASK-GAMEPLAY-CB-006` 当前最新事实：producer 阶段评审已完成，并正式决定当前继续保持 `internal_playable_alpha_late`；统一 gate `pass` 作为阶段可选升级前提保留，但本轮不升级为 `closed_beta_candidate`。
  - `TASK-GAMEPLAY-CB-007` 当前最新事实：对外口径已从 `technical preview / not playable yet` 调整为 `limited playable technical preview`，用于表达“可有限体验，但仍非 closed beta / live now”。
