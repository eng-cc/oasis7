# Gameplay 10 分钟留存修复计划（2026-04-09） PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md`

审计轮次: 1

## 1. Executive Summary

- Problem Statement: 当前 oasis7 已具备“偶尔能好玩”的核心，但 10 分钟留存仍不稳定。最近样本同时存在 `PostOnboarding`/pure API `4/5` 的正反馈，以及 `software_safe Step x1 completed_timeout`、首连脆弱、首屏 operator/debug 语义过重带来的 `2/5` 阻断反馈，说明问题不在“完全没乐趣”，而在“最小控制地板、中循环承接和奖励节奏没有稳定成立”。
- Proposed Solution: 新增 `PRD-GAME-012`，把未来两周的玩法收口重点冻结为 5 条高优先级 lane：首次进入与最小控制地板、`PostOnboarding` 后 10 分钟工业中循环、首屏降噪与玩家身份收束、后果可见化与奖励节奏、QA 10 分钟留存门禁。
- Success Criteria:
  - SC-1: fresh bundle headed Web/UI 主路径 `open -> connected -> play/step/select` 在最近 5 个样本中首次成功率 `>= 95%`，且不再依赖手动 reopen/reload 才能进入可控态。
  - SC-2: 玩家在同一会话 10 分钟内可稳定完成“首座工厂单元 -> 首个制成品 -> 停机恢复 -> 第一次扩产取舍”四段工业中循环包。
  - SC-3: Player 首屏默认只突出“我是谁 / 当前主目标 / 主要阻塞 / 下一步建议”，与当前目标无关的 operator/debug/历史噪音不再抢占主焦点。
  - SC-4: 关键决策必须前台可见：玩家能在主界面看到至少 `已接受`、`执行中`、`已产出/已恢复`、`代价/阻塞` 四类反馈，而不是依赖日志或原始事件文本自行推断。
  - SC-5: `qa_engineer` 新增 10 分钟留存门禁后，最近 3/5 个 active-LLM 正式游玩样本应达到“整体有趣程度 >= 4 / 再玩一局意愿 >= 4 / 关键操作链路完整”的 `continue playing` 结论；`--no-llm` 仅保留为 debug/probe lane，不计入正式留存放行。

## 2. User Experience & Functionality

- User Personas:
  - `producer_system_designer`: 需要把“接下来两周到底先修什么”冻结成可执行任务，而不是继续泛化功能愿景。
  - `viewer_engineer`: 需要明确当前首要任务是主路径可信度、主目标表达与噪音治理，而不是继续扩展工具态能力。
  - `runtime_engineer`: 需要明确哪些控制语义、工业状态与恢复路径必须先成为稳定能力地板。
  - `qa_engineer`: 需要把“能跑通”升级为“愿意继续玩”的门禁，而不是只看单条 smoke 是否通过。
  - `agent_engineer`: 需要在不扩散范围的前提下，配合把 Agent 反馈语义收成更像“指挥结果”而不是“调试回执”。
- User Scenarios & Frequency:
  - 两周留存修复冲刺：每日一次 owner 对账。
  - headed Web/UI 首局复跑：每个候选改动至少 1 次。
  - active-LLM 正式 10 分钟会话：每个候选版本至少 3 次样本。
  - `--no-llm` 工业调试 lane：仅在工业链条或 Viewer 语义回归时使用，不单独形成正式可玩性结论。
- User Stories:
  - PRD-GAME-012: As a 中循环玩家与制作人 owner, I want a stable 10-minute retention loop after onboarding, so that the game feels like a playable strategy experience instead of an occasionally compelling debug surface.
- Critical User Flows:
  1. Flow-RR-001: `玩家首次进入 -> connected -> 完成最小控制动作 -> 确认世界已可靠响应`
  2. Flow-RR-002: `玩家完成 PostOnboarding -> 建成首座工厂 -> 拿到首个制成品 -> 遭遇并恢复一次阻塞 -> 获得第一次扩产/分支建议`
  3. Flow-RR-003: `玩家看到代价/阻塞/恢复/奖励 -> 判断“这一步值得继续追” -> 继续留在当前会话`
  4. Flow-RR-004: `QA 汇总 10 分钟留存卡片 -> 与 runtime/viewer 指标对账 -> producer 决定 continue / hold`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 首次控制地板 | `first_control_success`、`ttfc_ms`、`control_hit_rate`、`requires_manual_recover` | headed Web/UI 主路径 `play/step/select` | `blocked -> unstable -> stable` | 首次成功率优先于平均成功率；任何手动恢复都记为失败样本 | `viewer_engineer` / `runtime_engineer` 共同 owner，`qa_engineer` 复核 |
| 10 分钟工业中循环包 | `factory_ready`、`first_product`、`blocked_reason`、`resumed`、`branch_offer` | 建厂、投产、恢复、扩产/转运取舍 | `post_onboarding -> industrial_bootstrap -> resilient_production -> branch_ready` | 必须先有持续产能，再给扩产/治理/冲突分支 | `producer_system_designer` 冻结口径，`runtime_engineer` / `viewer_engineer` 落地 |
| 首屏噪音治理 | `primary_goal_visible`、`noise_competes_with_goal`、`player_identity_clarity` | 默认首屏仅突出主目标与下一步 | `toolish -> noisy_playable -> player_focused` | 当前目标相关信息优先于历史噪音、operator/debug 信息 | `viewer_engineer` owner |
| 后果可见化 | `accepted`、`executing`、`produced_or_resumed`、`cost_or_blocker` | 主 HUD / toast / chatter 反馈 | `implicit -> readable -> motivating` | 先解释关键代价与结果，再展示次要日志 | `viewer_engineer` owner，`agent_engineer` 配合语义 |
| 10 分钟留存 gate | `fun_score`、`replay_intent`、`ten_minute_completion`、`lane` | playability card + release gate 汇总 | `draft -> watch -> continue_playing / hold` | 仅 active-LLM lane 可形成正式结论；`--no-llm` 只作 debug baseline | `qa_engineer` owner，`producer_system_designer` 最终裁决 |

- Acceptance Criteria:
  - AC-1: `PRD-GAME-012` 明确未来两周只优先做 5 条 lane，不把战争/治理/元进度扩面写进当前冲刺主目标。
  - AC-2: `gameplay` 主文档与 `game` 根 PRD/project 挂载 `PRD-GAME-012`，并给出 `TASK-GAME-061~065` 映射。
  - AC-3: 至少 1 个专题 project 明确写出每条 lane 的 owner role、test tier、验收命令与 done definition。
  - AC-4: 正式 10 分钟留存 gate 必须区分 active-LLM 正式游玩与 `--no-llm` debug/probe lane，避免口径回退到 observer-only 样本。
  - AC-5: `software_safe` / headed Web/UI 中任一正式入口若仍存在 `control ack timed out without progress` 这类最小控制地板失败，则本专题默认保持 `hold`。
  - AC-6: 本专题必须给出“该砍什么 / 该补什么 / 两周排期”三类裁决，而不是泛化成长期愿景。
  - AC-7: execution log、根入口与专题 project 的当前阶段判断必须继续保持 `internal_playable_alpha_late`，不借本专题提前放宽 `closed beta` 口径。
- Non-Goals:
  - 不在本专题中新增战争、治理、元进度的大范围新功能。
  - 不把 Prompt Ops / operator-only 能力重新包装成默认玩家主路径。
  - 不把 `--no-llm` 调试 lane 重新定义为正式游玩入口。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - `agent-browser` headed Web/UI 10 分钟亲玩与截图/录屏。
  - active-LLM 正式留存样本。
  - deterministic `--no-llm` 工业调试 lane。
- Evaluation Strategy:
  - 使用 MDA 口径：先修 Mechanics 的控制可信度，再让 Dynamics 形成 10 分钟中循环，最后看 Aesthetics 是否达到“愿意再玩一局”。

## 4. Technical Specifications

- Architecture Overview:
  - `PRD-GAME-012` 不新增底层玩法协议，而是收束下一个两周窗口的产品/体验优先级。
  - `viewer_engineer` 负责首屏、噪音、反馈与正式入口表达。
  - `runtime_engineer` 负责控制语义、工业状态/阻塞/恢复 canonical 化与能力地板。
  - `qa_engineer` 负责把 10 分钟留存卡升级为正式门禁。
  - `producer_system_designer` 负责范围控制、阶段裁决与跨角色排序。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-top-level-design.project.md`
  - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
  - `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
  - `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
  - `doc/playability_test_result/playability_test_card.md`
  - `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - active-LLM lane 通过但 software_safe 正式入口仍失败：记为 retention blocker，不得只用标准 3D 路径冲淡最弱入口缺陷。
  - 工业链条能推进但主界面没有“阻塞/恢复/产出”语义：记为中循环不成立，而不是纯 UI 瑕疵。
  - 首屏主目标存在但被历史 `AgentNotFound`、operator 面板或 raw snapshot 语义抢焦点：记为玩家身份失败。
  - `--no-llm` 工业调试链路通过但 active-LLM 正式样本失败：正式 gate 仍为 `hold`。
- Non-Functional Requirements:
  - NFR-RR-1: 10 分钟正式留存 gate 的 active-LLM 样本必须在 fresh bundle 本地可复跑。
  - NFR-RR-2: 任一正式入口的首次控制成功率低于 `95%` 时，不得宣称“10 分钟留存主循环已收口”。
  - NFR-RR-3: 每条 lane 的结论必须在同日回写到 task execution log 与对应 PRD/project。
  - NFR-RR-4: 所有结论继续遵守 `internal_playable_alpha_late` / `internal_only` claim envelope，不借体验改进任务扩大对外承诺。
- Security & Privacy:
  - 本专题只调整玩法优先级、体验反馈与门禁口径，不新增玩家敏感数据采集。

## 5. Risks & Roadmap

- Phased Rollout:
  - R0: 冻结 `PRD-GAME-012` 与 5 条 lane，停止需求蔓延。
  - R1: 完成首次控制地板与首屏降噪收口。
  - R2: 完成 10 分钟工业中循环包与后果可见化。
  - R3: 跑 active-LLM 10 分钟留存 gate，并由 producer 决定 continue / hold。
- Technical Risks:
  - 风险-1: 只修 UI 降噪，不修控制地板，会让“更像游戏”的界面放大失败体验。
  - 风险-2: 只修 deterministic `--no-llm` lane，会再次把正式游玩结论建立在 debug 样本上。
  - 风险-3: 继续往首屏加系统卖点，会稀释当前最需要的 10 分钟中循环密度。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-012 | `TASK-GAME-061` | `test_tier_required` | 文档治理检查、根入口/专题入口/任务映射/执行日志互链核验 | 两周优先级与跨角色 owner 一致性 |
| PRD-GAME-012 | `TASK-GAME-062` | `test_tier_required` + `test_tier_full` | headed Web/UI 主路径复跑、software_safe 控制 floor 复核、控制成功率统计 | 首次控制可信度、正式入口稳定性 |
| PRD-GAME-012 | `TASK-GAME-063` | `test_tier_required` | 工业引导卡组 A/B/C + active-LLM 10 分钟样本 + branch-ready 人工复核 | `PostOnboarding` 后中循环承接与持续能力 |
| PRD-GAME-012 | `TASK-GAME-064` | `test_tier_required` | 首屏截图对比、Mission HUD/summary/toast/chatter 语义人工复核、噪音抢焦点评估 | 玩家身份、后果可见化、奖励节奏 |
| PRD-GAME-012 | `TASK-GAME-065` | `test_tier_required` | 10 分钟留存卡片、QA 汇总与 producer 继续/暂停决策 | continue / hold 裁决与后续冲刺方向 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-RR-001 | 两周内只做 5 条高优先级 retention lane | 同时继续扩大战争/治理/元进度新功能 | 当前主要矛盾是 10 分钟留存，不是功能面不够大。 |
| DEC-RR-002 | active-LLM 样本作为正式 retention gate | 继续使用 `--no-llm` 作为正式可玩性结论 | 当前制作人口径已明确 no-LLM 仅保留 observer/debug。 |
| DEC-RR-003 | 先修控制地板，再做首屏 polish 与中循环加厚 | 先做更漂亮的前端或更宏大的系统宣传 | 玩家信任先于审美放大；不稳定控制会吞掉所有包装收益。 |
