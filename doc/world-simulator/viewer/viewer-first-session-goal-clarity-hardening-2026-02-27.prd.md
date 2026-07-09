# Viewer 首局目标清晰度加固（2026-02-27）

- 对应设计文档: `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.project.md`

审计轮次: 5

## 1. Executive Summary
- 将首局目标提示从“描述性文案”升级为“动作句 + 完成条件 + 预计耗时”，让玩家在 60 秒内明确第一步。
- 将首局信息架构改为“主任务优先、次任务折叠”，降低首次认知负担。
- 增加首局卡住检测与结算回顾，减少“世界在跑但我不知道下一步”的流失点。

## 2. User Experience & Functionality

### In Scope
- `crates/oasis7_viewer/src/egui_right_panel_player_guide.rs`
  - 主任务结构化文案（动作/条件/耗时）
  - 次任务折叠呈现
  - 主任务剩余量提示
- `crates/oasis7_viewer/src/egui_right_panel_player_experience.rs`
  - 首局进度状态跟踪
  - 5 秒无进展卡住检测
  - 首局完成结算卡片
- `crates/oasis7_viewer/src/egui_right_panel_player_*_tests.rs`
  - 新增/更新首局任务结构、卡住检测、结算触发单测

### Out of Scope
- Runtime/world 规则改动。
- 新增玩法系统（战争/治理/经济）协议。
- 大范围 UI 重排或视觉主题重构。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- 首局任务快照扩展（Mission HUD 内部）：
  - `next_action`
  - `completion_condition`
  - `eta`
  - `remaining_hint`
  - `target_frag_id`
  - `expected_material_hint`
  - `starter_value_reason`
  - `distance_or_accessibility_reason`
  - `first_recipe_relevance`
- 首局进度状态（Player Experience Local State）：
  - `session_started_at_secs`
  - `last_progress_tick`
  - `last_progress_event_count`
  - `last_progress_at_secs`
  - `stuck_hint_visible`
  - `summary_visible`
- 触发规则：
  - 卡住：连接成功后 `tick` 与 `event_count` 连续 5 秒无增量。
  - 结算：`guide_progress.explore_ready == true` 后首次展示回顾面板。
- 首局 frag 推荐规则：
  - 当 `next_action` 指向采集/探索某个 starter frag 时，Mission HUD 必须在行动前展示“为什么是这个 frag”的一行理由。
  - reason 至少覆盖材质预期、可达性原因和第一工业目标关联。
  - 若缺少这些字段，标记为 `starter_frag_hint_missing`，不得只展示泛化“探索附近碎片”。

## 5. Risks & Roadmap
- M1：主任务文案结构化（动作/条件/耗时）+ 次任务折叠。
- M2：下一步按钮语义统一（CTA 明确指向当前步骤）。
- M3：卡住检测与剩余量提示上线。
- M4：首局结算卡片与验收测试补齐。
- M5（2026-03-08）：修复“无进展提示”与新手引导卡在隐藏态同屏重叠问题，保持首屏引导可读性。
- M6（2026-03-18）：在 `4/4` 完成后切入正式 `PostOnboarding` 阶段目标，避免首局闭环结束后退回静态提示。

### Technical Risks
- 首局提示信息过多导致 HUD 再次拥挤。
  - 缓解：次任务默认折叠，仅主任务常显。
- 卡住检测误报（低事件密度场景被误判）。
  - 缓解：同时观察 `tick` 与 `event_count`，仅在连接成功后启用。
- 结算卡片打断沉浸。
  - 缓解：一次性展示，支持立即关闭。

## 验收口径
- Q1（目标理解）主观档位从“有点模糊”提升到“基本清楚/很清楚”为主。
- 首局 60 秒内玩家完成首个有效动作（打开面板/选择目标/触发反馈）的可观测比例提升。
- 首局推荐采集 frag 时，玩家在行动前能读到材质预期与它支持的第一工业目标；否则不计入目标理解通过。
- 卡住场景下 5 秒内出现明确恢复提示，不再静默等待。
- 当 `stuck_hint_visible=true` 且新手引导卡可见时，Mission HUD 不得与引导卡发生视觉重叠（同一截图中卡片边界有明确间距）。

## 完成态（2026-02-27）
- Mission HUD 已支持：
  - 动作句主目标
  - 完成条件
  - 预计耗时
  - 剩余目标提示
  - starter frag 材质预期与第一工业目标关联（后续实现项）
  - 次任务默认折叠
- “执行下一步”按钮已按步骤提供语义化 CTA，探索阶段可直接切换 Command 并触发 `play`。
- 已新增 5 秒无进展检测与恢复提示（基于 `tick + event_count` 双信号）。
- 已新增首局结算卡片（时长、tick 增量、事件增量、下一步建议）并支持关闭。
- 定向测试已覆盖 mission/stuck/summary 三组行为断言，viewer 编译检查通过。

## 增量需求（2026-03-08）
- PRD-ID: `PRD-VIEWER-FSGC-005`
- Problem Statement:
  - 左侧“检测到 X 秒无进展”提示触发时，Mission HUD 高度变高；在隐藏态同屏存在新手引导卡时，二者出现垂直重叠，导致文案互相遮挡。
- Proposed Solution:
  - 对 Mission HUD 锚点加入“无进展提示可见”条件分支，在 `panel_hidden && onboarding_visible && stuck_hint_visible` 场景下进一步下移锚点，确保与引导卡稳定分离。
- Functional Constraints:
  - 不改变 `play/pause/step` 恢复动作语义与按钮行为。
  - 不改变 `5 秒无进展` 判定阈值与诊断文案生成逻辑。
  - 仅调整布局锚点计算与其纯函数测试。
- Acceptance Criteria:
  - AC-FSGC-005-1: `player_mission_hud_anchor_y` 在隐藏态 + onboarding 可见 + stuck 提示可见时返回值大于“仅 onboarding 可见”场景。
  - AC-FSGC-005-2: 保持其他场景锚点不回退（展开态/无 onboarding/无 stuck）。
  - AC-FSGC-005-3: `test_tier_required` 定向单测通过，覆盖新增分支。
- 完成态（2026-03-08）:
  - `player_mission_hud_anchor_y` 已扩展为三参（`panel_hidden/onboarding_visible/stuck_hint_visible`），并在 `stuck_hint_visible=true` 时下移锚点避免与 onboarding 卡叠压。
  - 已通过 `player_mission_hud_` 与 `stuck_hint_` 定向测试，验证新增分支及既有无进展判定语义无回退。

## 增量需求（2026-03-18）
- PRD-ID: `PRD-VIEWER-FSGC-006`
- Problem Statement:
  - 当前首局 `4/4` 完成后只显示一次性 summary 与静态 continue 提示，Mission HUD 仍停留在 “Build Action Loop / 4/4”，没有把玩家正式接到 `PostOnboarding` 阶段目标。
- Proposed Solution:
  - 在 `guide_progress.explore_ready` 达成后，将 Mission HUD 切换为 `PostOnboarding` 阶段视图，默认围绕“建立第一项持续工业能力”生成主目标；summary 改为“进入下一阶段”语义，并隐藏已完成的 onboarding / 轻提示卡。
- Functional Constraints:
  - 不新增 runtime 规则或 quest engine。
  - 阶段目标必须复用现有 world event / runtime economy event / `lastControlFeedback` 信号，保持确定性与可测试性。
  - `PostOnboarding` 至少展示阶段目标、进度、阻塞或建议下一步三类信息。
- Acceptance Criteria:
  - AC-FSGC-006-1: `guide_progress.explore_ready == true` 后，Mission HUD 不再显示 “Mission: Build Action Loop” 作为主标题。
  - AC-FSGC-006-2: `PostOnboarding` 视图必须根据现有事件/反馈信号输出 `Active / Blocked / Branch Ready` 之一，并附带阶段进度。
  - AC-FSGC-006-3: 若存在工厂停机或 `completed_no_progress / blocked` 反馈，HUD 必须给出阻塞解释与建议下一步。
  - AC-FSGC-006-4: 首局 summary 的 CTA 与 `next_tip` 必须改为“进入下一阶段 / 建立持续能力”，不再停留在纯 continue 文案。
  - AC-FSGC-006-5: `test_tier_required` 定向单测通过，至少覆盖默认目标、阻塞归因、分支解锁与 summary 文案更新。
- 完成态（2026-03-18）:
  - `render_player_mission_hud` 已在 `explore_ready` 后切到 `PostOnboarding` 阶段卡，基于工业事件与控制反馈输出默认目标、阻塞原因、下一步建议与分支解锁提示。
  - onboarding 卡与轻提示在 `4/4` 后不再继续占据首屏；首局 summary 的标题、按钮与下一步文案已改成“进入下一阶段 / 建立持续工业能力”语义。
  - 已通过 `player_mission_tests::` 与 `player_summary_tests::` 定向回归。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- Test Plan & Traceability:
  - `PRD-VIEWER-FSGC-005 -> T5 -> test_tier_required`
  - `PRD-VIEWER-FSGC-006 -> T6 -> test_tier_required`
- Decision Log:
  - 选型：使用锚点下移解耦卡片重叠，保持信息完整显示。
  - 拒绝方案 A：隐藏 stuck 提示（会损失恢复引导价值）。
  - 拒绝方案 B：压缩引导卡内容（会降低首局任务可读性）。
  - 选型：用规则型 `PostOnboarding` 阶段卡承接 `4/4` 之后的下一步，而不是继续沿用一次性 summary + 静态提示。
