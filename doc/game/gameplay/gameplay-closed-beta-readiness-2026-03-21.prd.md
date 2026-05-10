# Gameplay 封闭 Beta 准入门禁（2026-03-21） PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`

审计轮次: 1

## 1. Executive Summary

- Problem Statement: 当前 oasis7 已经跨过“原型可演示”阶段，但各模块对项目所处阶段的判断、对外口径和下一阶段准入门禁仍分散在专题证据中，容易出现内部判断过高或对外承诺过早。
- Proposed Solution: 明确当前阶段为“内部可玩 Alpha 后段”，新增独立 `PRD-GAME-009` 统一定义冲刺 `封闭 Beta` 所需的准入包、角色 owner、失败回退条件与对外口径边界。
- Success Criteria:
  - SC-1: `game` 根入口、`gameplay` 主项目文档和 handoff 文档在同一轮回写中统一当前阶段口径，不再出现“已 Beta / 未到 Beta”的文档分叉。
  - SC-2: headed Web/UI、pure API、no-UI live protocol、longrun/recovery 四类证据收敛到同一套 `closed_beta_candidate` release gate，而不是分散专题各自 `pass`。
  - SC-3: `qa_engineer` 维护的阶段趋势在最近 5 个样本中达到首次通过率 `>= 60%`、阶段内逃逸率 `<= 20%`，否则不得升级阶段口径。
  - SC-4: 同一候选版本必须在 fresh bundle 本地复跑通过 headed Web/UI required、pure API required、no-UI smoke 和 replay/rollback drill 四类门禁。
  - SC-5: `viewer_engineer` 必须把 `PostOnboarding` 首屏收成“主目标优先、无无关噪音”的玩家入口；若首条路径仍出现与主目标无关的高显著错误噪音，则不得给出 `封闭 Beta` 体验结论。
  - SC-6: `liveops_community` 必须先完成封闭 Beta 候选 runbook、招募/反馈/事故回流模板与对外禁语清单；在 `producer_system_designer` 最终放行前，对外仍保持 `limited playable technical preview` 口径。

## 2. User Experience & Functionality

- User Personas:
  - `producer_system_designer`: 需要把“当前阶段是什么、何时可升级阶段”落成正式门禁，而不是口头判断。
  - `runtime_engineer`: 需要明确哪些长期在线、恢复、回滚和发布阻断证据是冲 Beta 的硬门。
  - `viewer_engineer`: 需要知道当前最优先的是玩家入口收口，而不是继续泛化工具性能力。
  - `qa_engineer`: 需要把分散专题证据串成统一 release gate，并给出升阶/不升阶结论。
  - `liveops_community`: 需要清楚当前对外仍只能说什么、不能说什么，以及何时才允许切换对外口径。
- User Scenarios & Frequency:
  - 阶段评审：每次关键专题收口或候选版本前至少 1 次。
  - 候选版本放行：每个 `closed_beta_candidate` 版本至少 1 次。
  - 运营口径校准：每次对外预热、招募或渠道反馈前执行。
- User Stories:
  - PRD-GAME-009: As a 制作人与阶段评审 owner, I want one auditable closed-beta admission gate, so that the team upgrades stage claims only when runtime, viewer, QA and liveops all agree on the same evidence.
- Critical User Flows:
  1. Flow-CB-001: `制作人读取当前专题与证据 -> 判定当前阶段为 internal_playable_alpha_late -> 冻结下一阶段为 closed_beta_candidate`
  2. Flow-CB-002: `runtime/viewer/QA/liveops 各自完成收口任务 -> 产出证据 -> QA 汇总统一 release gate`
  3. Flow-CB-003: `任一关键门禁失败 -> 保持 internal_playable_alpha_late -> 记录阻断与 owner -> 不升级对外口径`
  4. Flow-CB-004: `全部门禁通过 -> producer 组织阶段评审 -> 若通过则升级到 closed_beta_candidate -> liveops 更新允许口径`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 当前阶段判定 | `current_stage`、`candidate_stage`、`claim_envelope`、`decision_date` | 制作人发起阶段评审并冻结当前阶段口径 | `prototype -> internal_playable_alpha -> internal_playable_alpha_late -> closed_beta_candidate -> closed_beta` | 先内部阶段，再候选阶段，再对外阶段 | 仅 `producer_system_designer` 可修改正式阶段判定 |
| runtime 准入包 | `soak_run_id`、`replay_drift_status`、`rollback_drill_status`、`release_gate_status` | 收集长跑、恢复、回放、阻断接口证据 | `missing -> collected -> verified -> blocking/non_blocking` | 先 required，再 full，再连续通过 | `runtime_engineer` 负责生成，`qa_engineer` 负责复核 |
| Viewer 玩家入口收口 | `post_onboarding_noise_free`、`first_screen_focus`、`full_coverage_gate_status` | 清理噪音、提升主目标优先级、补齐玩家入口 gate | `debuggable -> playable -> candidate_ready -> blocked` | 先核心首屏，再全覆盖 gate | `viewer_engineer` 负责实现，`qa_engineer` 负责验收 |
| 统一 release gate | `web_required`、`pure_api_required`、`headless_smoke`、`longrun_required`、`trend_status` | QA 汇总并给出 `pass/block` 结论 | `draft -> running -> pass/block` | 任一硬门失败即整体 `block` | `qa_engineer` 可独立给阻断建议，`producer_system_designer` 最终拍板 |
| 对外口径与运营准备 | `external_phrase_allowlist`、`recruitment_ready`、`incident_loop_ready`、`feedback_route_ready` | 生成 runbook、FAQ、升级路径与禁语清单 | `preview_only -> candidate_ready -> beta_messaging_ready` | 先准备 SOP，再放宽口径 | `liveops_community` 负责执行，`producer_system_designer` 批准升级口径 |

- Acceptance Criteria:
  - AC-1: `PRD-GAME-009` 明确当前阶段、下一阶段目标、失败回退条件与对外口径边界。
  - AC-2: `doc/game/project.md` 中新增 owner 清晰的跨角色任务，且每个任务都可映射到 `runtime / viewer / QA / liveops` 之一。
  - AC-3: headed Web/UI、pure API、no-UI live protocol、longrun/recovery 四类证据必须进入同一 release gate，而不是散落在各专题“各自 pass”。
  - AC-4: 若 QA 趋势未达到首次通过率 `>= 60%`、阶段内逃逸率 `<= 20%`，则必须在结论中保留 `internal_playable_alpha_late`，不得升级阶段。
  - AC-5: `PostOnboarding` 首屏必须以阶段目标、进度、阻塞、下一步建议为主语义；若继续出现与主目标无关的高显著错误噪音，则 `viewer` 体验门禁失败。
  - AC-6: 对外文档与 runbook 在 producer 放行前只能使用 `limited playable technical preview` 一类口径，不得出现 `closed beta` / `play now` / `live now`。
  - AC-7: 当前阶段结论、任务拆解、handoff、devlog 必须在同一轮文档回写中完成，确保角色协作可追溯。
- Non-Goals:
  - 不在本专题中直接新增完整商业化包装或公开发行计划。
  - 不把 LLM 模式专属动作等价纳入本轮准入硬门。
  - 不在本轮重新定义工业、治理、战争的底层规则。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - `agent-browser` Web 闭环回归与截图/录屏产物。
  - 纯 API smoke / parity 脚本。
  - longrun / replay / rollback / release gate 脚本。
- Evaluation Strategy:
  - 用“统一 gate 是否可复跑”“趋势指标是否达标”“玩家入口是否降噪”“运营口径是否收口”四条线共同评估，而不是仅看单一专题通过。

## 4. Technical Specifications

- Architecture Overview:
  - `PRD-GAME-009` 不新增新玩法规则，而是定义一层跨专题的准入治理层。
  - `runtime_engineer` 负责长期在线与恢复可信度证据。
  - `viewer_engineer` 负责玩家首屏与 full-coverage gate 的体验面收口。
  - `qa_engineer` 负责统一 gate、趋势判定和阻断结论。
  - `liveops_community` 负责 runbook、招募/反馈/事故回流与对外口径边界。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/gameplay/gameplay-top-level-design.project.md`
  - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
  - `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
  - `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
  - `doc/testing/evidence/post-onboarding-headless-smoke-2026-03-19.md`
  - `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 单一专题 `pass` 但统一 gate 缺证：必须维持 `internal_playable_alpha_late`，并在项目文档标注缺失 lane。
  - source tree 通过但 fresh bundle 失败：必须按候选版本阻断处理，不得用 source 结果替代。
  - viewer 首屏主目标存在但被高显著噪音淹没：记为体验门禁失败，而不是仅记 UI 瑕疵。
  - 趋势样本不足：允许继续收样，但不得提前宣布升级阶段。
  - 运营渠道被要求提供“可玩链接 / Beta”说法：必须回退到禁语清单与 GitHub 反馈路线，不得临时放宽承诺。
  - LLM 模式能力与 no-LLM required 结果混淆：必须在 gate 中分 lane 标注，避免用 LLM 特例冲淡当前准入口径。
- Non-Functional Requirements:
  - NFR-CB-1: 统一 release gate 的所有命令和产物路径必须可在 fresh bundle 本地复跑。
  - NFR-CB-2: `qa_engineer` 的最近 5 个阶段样本中首次通过率 `>= 60%`，阶段内逃逸率 `<= 20%`。
  - NFR-CB-3: 同一候选版本必须至少具备 1 轮 headed Web/UI required、1 轮 pure API required、1 轮 no-UI smoke、1 轮 replay/rollback drill 的通过证据。
  - NFR-CB-4: 长跑和恢复类失败若被标记为 `blocking`，必须在下一轮阶段评审前关闭或显式降级目标，不允许“带阻断升级阶段”。
  - NFR-CB-5: 对外文档、runbook 与 devlog 中的阶段口径在 1 个工作日内同步。
  - NFR-CB-6: 在 producer 最终放行前，公开渠道 100% 使用 `limited playable technical preview` 口径。
- Security & Privacy:
  - 本专题仅汇总证据、沟通和阶段判断，不新增新的玩家敏感数据采集。
  - 对外反馈样本与事故摘要需遵循最小披露原则，避免在公开渠道泄露内部世界状态或用户敏感信息。

## 5. Risks & Roadmap

- Phased Rollout:
  - MVP: 冻结当前阶段口径、统一任务拆解与 handoff。
  - v1.1: 跑通统一 `closed_beta_candidate` release gate，拿到第一轮完整证据包。
  - v2.0: 当趋势、体验、longrun 和 liveops 四条线都达标后，再决定是否升级到 `closed_beta_candidate` 或继续保持 `internal_playable_alpha_late`。
- Technical Risks:
  - 风险-1: 继续沿用专题各自 `pass` 的做法，会让团队误判为“已经 Beta”。
  - 风险-2: runtime 与 viewer 各自通过，但 fresh bundle 统一候选版本未真正跑通。
  - 风险-3: liveops 提前放宽对外口径，导致真实承诺超出当前产品成熟度。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-009 | `TASK-GAME-028` | `test_tier_required` | 文档治理检查、根入口/索引/handoff/devlog 互链核验 | 阶段口径、追踪一致性 |
| PRD-GAME-009 | `TASK-GAME-029` | `test_tier_required` + `test_tier_full` | five-node no-LLM soak、replay/rollback drill、release gate 证据 | 长期在线、恢复、发布阻断可信度 |
| PRD-GAME-009 | `TASK-GAME-030` | `test_tier_required` | headed Web/UI 首屏回归、playability 卡片、viewer full-coverage gate 玩家入口抽样 | 玩家入口、`PostOnboarding` 表达、视觉噪音 |
| PRD-GAME-009 | `TASK-GAME-031` | `test_tier_required` + `test_tier_full` | 统一 release gate、趋势基线对账、fresh bundle lane 汇总 | 阶段放行、QA 阻断、候选版本可信度 |
| PRD-GAME-009 | `TASK-GAME-032` | `test_tier_required` | runbook/禁语清单/反馈回流模板检查 | 对外口径、招募与事故回流 |
| PRD-GAME-009 | `TASK-GAME-033` | `test_tier_required` | producer 阶段评审记录、go/no-go 结论、文档回写 | 最终阶段判断、跨角色裁决 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-CB-001 | 把当前阶段定为 `internal_playable_alpha_late` | 直接把当前阶段写成 `closed beta` | 现有证据已证明“不是原型”，但 QA 趋势、viewer 产品化和 liveops 对外口径仍未达到 Beta。 |
| DEC-CB-002 | 新增独立 `PRD-GAME-009` 作为冲封闭 Beta 的准入专题 | 继续把阶段判断散落在 devlog、评审结论和单个专题里 | 独立专题更利于统一 owner、证据与禁语边界。 |
| DEC-CB-003 | 先做统一 release gate，再考虑升级对外口径 | 允许专题各自 `pass` 后直接宣称 Beta | 统一 gate 才能避免“每块都差不多，但整体验证没真正跑通”的错觉。 |
