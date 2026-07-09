# Viewer 首局目标清晰度加固设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.project.md`

## 1. 设计定位
定义首局目标清晰度加固方案：把首局提示改造成“动作句 + 完成条件 + 预计耗时”的主任务视图，并补上卡住检测与结算回顾，降低新玩家首分钟迷失率。

## 2. 设计结构
- 目标表达层：统一主任务、次任务和完成条件结构，压缩首屏认知负担。
- starter frag 理由层：当首局推荐采集 frag 时，解释材质预期、可达性和第一工业目标关联。
- 引导展示层：首局 HUD 与右侧说明区共享同一套任务语义，避免多处文案冲突。
- 卡住检测层：根据玩家长时间无推进、无操作或重复失败信号触发提示升级。
- 回顾闭环层：在首局结束时输出阶段回顾，帮助玩家理解“做了什么、下一步是什么”。

## 3. 关键接口 / 入口
- `egui_right_panel_player_guide.rs`
- `egui_right_panel_player_experience.rs`
- 首局任务/回顾 HUD 文案模型
- starter frag reason fields: `target_frag_id`、`expected_material_hint`、`starter_value_reason`、`distance_or_accessibility_reason`、`first_recipe_relevance`
- 卡住检测与结算回顾入口

## 4. 约束与边界
- 本专题只优化首局信息架构与目标表达，不改玩法规则和运行时数值。
- 主任务优先、次任务折叠是默认展示原则，不追求一次展示全部系统信息。
- 卡住检测只能提供解释与建议，不替代玩家决策或自动执行控制。
- 目标文本必须保持短句、可执行、可验证，避免重新回到描述性叙事。
- starter frag 理由必须保持一行可读，不展开完整资源分布表或精确掉落承诺。

## 5. 设计演进计划
- 先冻结主任务/次任务/完成条件文案结构。
- 再接入首局 HUD、卡住检测和结算回顾。
- 最后通过体验测试与文档回写收口首局目标链路。

## 6. 增量设计（2026-03-18）
- 在 `guide_progress.explore_ready` 达成后，首局 HUD 不再继续扮演 onboarding 任务卡，而是切换为 `PostOnboarding` 阶段卡。
- 阶段卡不引入新 runtime 状态机，直接消费既有工业事件、runtime economy 事件与 `lastControlFeedback`：
  - 默认目标：建立第一项持续工业能力。
  - 阻塞态：优先解释工厂停机、无推进或控制阻塞。
  - 分支就绪态：当首个产出/稳定产线出现后，提示转入生产扩张 / 治理影响 / 冲突安全。
- onboarding 卡与轻提示在 `4/4` 后退场，避免“教程已完成但仍占据首屏”的语义冲突。
