# PostOnboarding 阶段目标链 PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.project.md`

审计轮次: 1

## 0. 设计摘要

### 0.1 目标

- 在首次行动闭环完成后，为玩家提供正式的 `PostOnboarding` 阶段，而不是停留在一次性总结提示。
- 将玩家目标从“学会发出一次行动”切换为“建立持续运转的组织能力”。
- 以工业成长为默认第一主线，并在首个持续能力里程碑后显式展开治理 / 协作 / 扩张分支；分支必须说明选择该路线会改变什么，而不是只给方向标签。

### 0.2 范围

#### In Scope
- `FirstSessionLoop -> PostOnboarding -> MidLoop` 的阶段衔接定义。
- `PostOnboarding` 阶段目标、状态、反馈、阻塞分类与分支规则。
- Viewer 中目标卡、阶段卡、阻塞反馈与阶段完成提示的产品口径。
- 对 `#46` 的产品级验收标准、测试入口与角色 owner 拆解。

#### Out of Scope
- 不在本期实现完整动态任务系统或无限 quest 生成器。
- 不在本期改写工业、治理或高风险对抗模块的底层数值与 runtime 规则。
- 不在本期强制锁定玩家路线，保持沙盒自由探索能力。

### 0.3 接口 / 数据

- 上游输入：
  - 首次行动闭环完成信号（`FirstSessionLoop completed`）
  - 工业里程碑状态、停机 / 恢复原因、治理 / 危机 / 合约 / 协作提示
  - 当前玩家组织的资源、产线、节点、目标对象状态
- 下游输出：
  - `PostOnboarding` 阶段状态（`introduced / active / blocked / milestone_completed / branch_ready`）
  - 主目标卡、次级机会卡、阻塞原因、建议下一步
  - 可玩性卡片与 `test_tier_required` / Web 闭环证据

### 0.4 里程碑

- M0 (2026-03-18): 冻结 `PostOnboarding` 阶段 PRD / design / project 与根入口追踪。
- M1: 打通 Viewer 阶段切换与单个主目标卡，完成工业优先的首个里程碑目标。
- M2: 接入阻塞原因分类、阶段完成提示与次级机会卡。
- M3: 打通 `PostOnboarding -> MidLoop` 分支承接与 playability / Web required-tier 验证。

### 0.5 风险

- 如果阶段目标仍然过于抽象，玩家会把它视为“教程尾声文案”，而不是下一阶段。
- 如果默认目标选择错误，玩家会被错误引导到不具备可行性的世界状态。
- 如果阻塞原因不可解释，玩家会重新落回“我不知道该干什么”的体验。
- 如果 Viewer 只显示目标，不显示进度与阻塞，`#46` 只会被表面缓解。

## 1. Executive Summary

- Problem Statement: 当前首轮 onboarding 在完成首次行动反馈后即结束，缺少把玩家导入下一阶段目标链的正式承接层，导致玩家会操作但不知道为什么继续玩、接下来追什么。
- Proposed Solution: 定义独立的 `PostOnboarding` 阶段，用一个工业优先、可持续能力导向的主目标，把玩家从“操作教学”正式接到“组织经营”，并在首个里程碑后开放治理 / 协作 / 扩张方向。
- Success Criteria:
  - SC-1: 玩家完成首次行动闭环后，同一会话内必须进入 `PostOnboarding` 阶段，不允许只显示一次性总结提示。
  - SC-2: 系统必须生成 1 个主目标和最多 2 个次级机会，且主目标 100% 具备显式状态 `active / blocked / completed` 之一。
  - SC-3: 主目标卡必须同时展示进度、阻塞原因和建议下一步；阻塞原因至少覆盖 `缺电 / 缺料 / 物流阻塞 / 治理限制 / 危机或协作中断` 五类。
  - SC-4: 第一阶段主目标应能在 1~3 次会话或 15~45 分钟有效操作内达成首个里程碑，不能直接跳到数周级长循环目标。
  - SC-5: 玩家达成首个持续能力里程碑后，系统必须切换到 `branch_ready`，并显式提示至少一个中循环方向（生产扩张 / 治理影响 / 协作恢复）及对应承诺条：即时收益、后续节拍变化、风险/锁定和下次会话 hook。

## 2. User Experience & Functionality

- User Personas:
  - 新玩家：刚完成首次行动闭环，需要知道“下一步做什么”与“怎样算变强”。
  - 回流玩家：重新进入世界时，需要快速恢复当前阶段目标与阻塞。
  - 玩法设计者：需要把“教程之后”的目标链收成统一设计口径。
  - `viewer_engineer` / `qa_engineer`: 需要明确字段、状态和验收门槛。
- User Scenarios & Frequency:
  - 首次会话：每位新玩家 1 次，完成 `4/4` 后立即触发。
  - 前 1~3 次回访：高频出现，用于建立首个持续能力里程碑。
  - 版本回归：每个候选版本至少 1 次，用于验证 `#46` 不再回退。
- User Stories:
  - PRD-GAME-007: As a 新玩家, I want a post-onboarding stage objective, so that I know what to build after the first guided action.
- PRD-GAME-007-A: As a 回流玩家, I want the system to remind me of my current stage goal and blocker, so that I can resume without having to relearn the interface.
  - PRD-GAME-007-B: As a 玩法设计者, I want onboarding completion to hand off into a measurable capability milestone, so that the first-session loop and mid-loop are connected.
  - PRD-GAME-007-C: As a QA, I want stable stage states and acceptance signals, so that `#46` can be verified with repeatable evidence.
- Critical User Flows:
  1. Flow-POD-001: `完成首次行动反馈 -> 系统判定 first-session complete -> 弹出 PostOnboarding 阶段卡 -> 激活主目标卡`
  2. Flow-POD-002: `玩家查看主目标 -> 阅读进度 / 阻塞 / 下一步 -> 执行相关操作 -> 目标状态更新为 active / blocked / completed`
  3. Flow-POD-003: `主目标被阻塞 -> 系统展示阻塞分类与建议修复动作 -> 玩家处理阻塞 -> 目标恢复 active`
  4. Flow-POD-004: `达成首个持续能力里程碑 -> 阶段状态切换为 branch_ready -> 系统展示生产 / 治理 / 协作方向建议 -> 玩家理解选择路线会改变什么`
  5. Flow-POD-005: `玩家中途离开 -> 下次进入世界 -> 恢复当前阶段目标、最近阻塞和下一步建议`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 阶段切换卡 | `stage_id`、`stage_title`、`entry_reason`、`primary_goal_id`、`time_hint` | 显示“进入下一阶段”；允许 `Focus Goal` 或收起；不提供跳过后永久消失 | `hidden -> introduced -> active` | 仅在 `FirstSessionLoop` 完成后触发 1 次；若已完成首个里程碑则不再出现 | 所有玩家可见；系统自动触发 |
| 主目标卡 | `goal_id`、`goal_type`、`goal_title`、`target_condition`、`current_progress`、`blocker_primary`、`next_step_hint` | 允许聚焦、展开详情、定位相关面板；不提供手动 reroll | `candidate -> active -> blocked -> completed -> handed_off` | v1 默认 `工业持续能力 > 生产恢复 > 交易/协作 > 治理/扩张`；同分时优先离当前状态最近的目标 | 玩家只读查看；系统根据世界状态选择 |
| 次级机会卡 | `goal_id`、`goal_type`、`why_now`、`risk_level` | 点击可查看，不替代主目标 | `hidden -> visible -> expired/selected` | 最多 2 个；用于提示后续方向，不覆盖主目标 | 玩家只读查看 |
| 阻塞反馈 | `blocker_type`、`blocker_reason`、`affected_target`、`suggested_fix` | 点击后展开原因与建议下一步 | `none -> flagged -> resolved` | 仅保留 1 个主阻塞，其余收纳到详情区 | 所有玩家可见；由系统计算 |
| 阶段完成与分支提示 | `milestone_id`、`milestone_result`、`branch_recommendations`、`route_label`、`immediate_gain`、`future_beat_changed`、`risk_or_lockin`、`next_session_hook` | 展示“你已建立第一项持续能力”；可查看推荐分支与每条路线的最小承诺条 | `active -> milestone_completed -> branch_ready` | 完成首个持续能力里程碑后触发；默认推荐 1~3 条中循环方向；标签不能替代路线后果说明 | 所有玩家可见；系统自动生成 |

- Acceptance Criteria:
  - AC-1: `FirstSessionLoop` 完成后必须出现正式阶段切换，而不是仅保留静态 “continue” 提示。
  - AC-2: `PostOnboarding` 主目标必须围绕“建立持续组织能力”，不能退化为再做一次相同点击动作。
  - AC-3: v1 默认首个主目标必须优先选择工业 / 产线 / 恢复型能力目标，除非当前世界状态明确不满足可行性。
  - AC-4: 每个主目标都必须可解释为 `为何是现在`、`当前进展`、`主要阻塞`、`建议下一步` 四段信息。
  - AC-5: 阻塞分类必须统一映射到 `缺电 / 缺料 / 物流阻塞 / 治理限制 / 危机或协作中断`；不得返回纯泛化失败文案。
  - AC-6: 玩家离开并回到世界后，必须能恢复当前阶段与最近一次主目标状态。
  - AC-7: 达成首个持续能力里程碑后，系统必须显式显示至少一个中循环方向，不允许重新回到“无目标自由漂浮”。
  - AC-8: `#46` 的回归验证必须具备 `test_tier_required` 的 Viewer / Web 证据与 playability 卡片引用。
  - AC-9: `branch_ready` 的 1~3 条推荐分支必须各自给出 `route_label / immediate_gain / future_beat_changed / risk_or_lockin / next_session_hook`；只有“扩产 / 治理 / 协作”标签而无后果说明时不得验收通过。
- Non-Goals:
  - 不做任意数量的动态分支树。
  - 不做完全个性化的 LLM 任务生成。
  - 不把玩家自由探索替换成强制线性任务链。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - v1 不要求 LLM 生成任务文本；目标推荐优先使用规则型选择器。
  - 可玩性评估继续依赖 `agent-browser` Web 闭环、截图 / 视频证据与 QA 卡片。
- Evaluation Strategy:
  - 用 `#46` 回归通过率、首次阶段目标达成率、玩家是否能说出“下一步干什么”的主观卡片反馈来评估。
  - 同步抽样验证目标选择是否与当前世界状态一致，避免推荐不可行目标。

## 4. Technical Specifications

- Architecture Overview:
  - 阶段切换由 `FirstSessionLoop completed` 信号触发。
  - 目标选择器读取工业运行态、组织状态、阻塞分类与微循环提示，产出 1 个主目标与最多 2 个次级机会。
  - Viewer 负责显示阶段卡、目标卡、阻塞反馈与阶段完成提示；runtime 继续提供状态与事件事实，不在 v1 直接承担 quest 逻辑。
- Integration Points:
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
  - `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
  - `crates/oasis7_viewer/src/egui_right_panel_player_experience.rs`
  - `crates/oasis7_viewer/src/egui_right_panel_player_guide.rs`
  - `crates/oasis7_viewer/src/egui_right_panel_player_micro_loop.rs`
- Edge Cases & Error Handling:
  - 无可行工业主目标：若当前世界状态没有可执行工业链路，回退到“恢复被阻塞产线”或“完成首次交易 / 协作”目标，但仍需保持“持续能力”导向。
  - 主目标在进入阶段前已满足：自动标记为 `completed`，并立即生成下一个主目标或进入 `branch_ready`。
  - 多个阻塞同时存在：主卡只展示 1 个主阻塞，其余在详情区列出，避免信息过载。
  - 状态快照过旧：若关键状态滞后，主目标卡显示 `syncing` 并暂停展示阻塞结论，避免误导。
  - 玩家忽略主目标：系统允许自由探索，但必须保留可重新聚焦的主目标卡，不因收起而永久消失。
  - 会话中断后恢复：重新进入时必须恢复 `stage_id / goal_id / blocker_primary / next_step_hint`。
  - 危机 / 协作中断导致主目标不可达：切换为 `blocked`，并优先给出“保全 / 恢复”型下一步建议，而不是继续催促原目标。
  - 分支只有方向标签：若 `branch_ready` 只展示生产扩张 / 治理影响 / 协作恢复等名称，而没有即时收益、后续节拍变化、风险/锁定和下次会话 hook，记为 `branch_commitment_missing`，不得当作中循环承接完成。
  - AgentNotFound 历史噪音：虽然 `RejectReason::AgentNotFound` 会留在历史聊天/控制拒绝中，这类噪音不得重复占据 Action Status 标题或右侧聊天区焦点，以便首次进入 `PostOnboarding` 时视线优先落在主目标与进度。
- Non-Functional Requirements:
  - NFR-POD-1: `FirstSessionLoop` 完成到 `PostOnboarding` 主目标首次可见的延迟目标为 P95 <= 1 秒（本地 Viewer 会话）。
  - NFR-POD-2: 同一世界状态输入下，v1 主目标选择结果必须保持确定性，避免同条件下随机变化。
  - NFR-POD-3: 100% 主目标必须具备显式状态、进度、阻塞、下一步建议四类字段。
  - NFR-POD-4: `PostOnboarding` 不能隐藏现有自由操作入口；系统给目标，但不锁死玩家。
  - NFR-POD-5: `#46` 回归验证在 `test_tier_required` 中必须可复跑，并具备至少 1 条 Web 证据和 1 张 playability 卡片。
  - NFR-POD-6: 相关变更在 1 个工作日内同步回写 `game` 根 PRD / project / 索引。
  - NFR-POD-7: `branch_ready` 首次出现时，100% 推荐分支必须具备最小承诺条；v1 不要求动态 quest tree，但要求玩家能比较“选这条路线会改变什么”。
- Security & Privacy:
  - 本期仅消费已有世界状态和事件快照，不新增账号级敏感数据采集。
  - 目标推荐不得绕过 runtime 权限边界，不得生成超出当前系统实际可执行能力的承诺。

## 5. Risks & Roadmap

- Phased Rollout:
  - MVP: 工业优先的单主目标 + 阻塞反馈 + 阶段完成提示。
  - v1.1: 次级机会卡 + 分支推荐（生产扩张 / 治理影响 / 协作恢复）。
  - v2.0: 跨会话阶段历史、更多世界状态驱动的目标选择与更细的中循环承接。
- Technical Risks:
  - 风险-1: 目标选择器过早依赖不稳定的 runtime 信号，导致推荐抖动。
  - 风险-2: Viewer 同时显示阶段卡、微循环卡、工业反馈卡，可能再次造成信息拥挤。
  - 风险-3: 如果验收只看 UI 存在，不看玩家是否真的得到下一步，就会把 `#46` 再次做成表层修复。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-007 | `TASK-GAME-021` + `TASK-GAMEPLAY-POD-001/002/003/004` | `test_tier_required` | 文档治理检查、Viewer 定向回归、Web 闭环、playability 卡片 | 新手阶段承接、Viewer 目标表达、`#46` 回归 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-POD-001 | 新增独立 `PostOnboarding` 阶段 | 继续沿用 onboarding 结束后的静态提示 | 静态提示不能形成正式阶段切换，也无法承接 mid-loop。 |
| DEC-POD-002 | 首个主目标默认工业 / 持续能力优先 | 直接把治理 / 对抗 / 长循环目标抛给新玩家 | 工业成长是当前最早可感知、最可验证的组织能力闭环。 |
| DEC-POD-003 | v1 采用规则型目标选择器 | 直接用 LLM 生成动态任务 | 规则型更稳定、可测试，也更适合先关闭 `#46` 的产品缺口。 |
| DEC-POD-004 | `branch_ready` 使用最小承诺条表达 1~3 条分支 | 只展示“扩产 / 治理 / 协作”等方向名称，或直接做完整动态任务树 | 方向标签不能形成战略承诺感；完整 quest tree 超出 v1 范围，最小承诺条能先支撑比较与回访动机。 |
