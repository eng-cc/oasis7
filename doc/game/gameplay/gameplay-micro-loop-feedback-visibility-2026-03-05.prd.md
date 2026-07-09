# Gameplay 微循环反馈可见性闭环（Runtime 协议 + Viewer 反馈）

- 对应设计文档: `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md`

## ROUND-002 主从口径
- 主入口：`doc/game/gameplay/gameplay-top-level-design.prd.md`
- 本文仅维护增量，不重复主文档口径。

## ROUND-003 增量目标（可玩性视觉优化）
- 在不改玩法规则本体的前提下，补齐“控制结果显著性 + 面板认知负载 + 世界可读性”三条体验链路。
- 以手动实操截图作为验收证据，确保优化结果可感知、可复核。

## 目标
- 修复微循环“动作被接受但无反馈/反馈延迟过长”的断裂，提升控制感与节奏体验。
- 补齐 Runtime 协议的“动作已接受”与 gameplay 事件可见反馈链路，避免玩家把系统延迟误判为“没听懂”。
- 建立可量化的微循环体验指标与回归口径，纳入发行门禁。
- 优先级来源: playability 卡片控制感评分偏低 + release gate 需要可验证微循环反馈。
- 降低玩家模式下的信息噪声，保证“下一步做什么”在首屏可读。
- 提升世界实体与选中目标的可读性，减少“场景空/反馈弱”的视觉落差。

## 范围

### In Scope
- Runtime 协议扩展：新增“动作接受确认”与可见化字段（action_id/action_kind/eta_ticks/notes）。
- Gameplay DomainEvent 可见性：战争/治理/危机/经济合约/元进度的事件映射与摘要规则。
- Viewer 反馈层：toast/事件行/微循环 HUD（倒计时 + 最近动作状态 + 无进展诊断）。
- Viewer 可见性与布局优化：控制结果显著条、玩家模式渐进披露、模块可见性默认策略修正。
- 玩家下一步动作可见性：formal summary / Viewer 主操作面必须把 canonical `available_actions` 转译成 1-3 个当前可执行或被阻塞的推荐行动。
- 世界可读性优化：选中强调与 2D marker 可见性/尺寸策略增强。
- 可玩性卡片指标回归：有效控制命中率、卡片 15/16/17 评分目标。
- 手动截图验收：每个迭代步骤输出截图并做视觉评估。

### Out of Scope
- 大规模玩法数值重平衡与规则重写（仅补反馈与协议可见性）。
- 3D 场景与美术资产改版。
- 网络/链上协议改造与跨节点共识流程。

## 接口 / 数据
- Runtime 新增事件（建议落为 DomainEvent）：
  - `ActionAccepted { action_id, action_kind, actor_id, eta_ticks, notes }`
- Gameplay DomainEvent 映射：`WarDeclared/WarConcluded/GovernanceProposalOpened/GovernanceVoteCast/GovernanceProposalFinalized/CrisisSpawned/CrisisResolved/CrisisTimedOut/EconomicContractOpened/Accepted/Settled/Expired/MetaProgressGranted`
- Viewer 显示字段：
  - 微循环 HUD：`last_action_status`、`pending_eta_ticks`、`due_timers`（war/governance/crisis/contract）
  - 反馈 toast：`tone + summary + action_id(optional)`

## 里程碑
- M1: Runtime 协议 ACK + Gameplay 事件映射与摘要规则。
- M2: 微循环 HUD 与无进展诊断提示可见。
- M3: 可玩性卡片回归达标并纳入发行门禁。

## 风险
- 事件映射口径不一致导致“反馈误导”，反而降低控制感。
- 新增 ACK 事件若节奏过密，可能造成信息噪声与 UI 干扰。
- 旧版本 runtime/viewer 兼容不足导致回退体验不一致。

## 1. Executive Summary

- **Problem Statement**: 当前微循环存在“动作被接受但缺少及时可见反馈”的体验断裂，导致控制感与节奏评分偏低，影响可玩性门禁判断。
- **Proposed Solution**: 在 runtime 协议中补齐动作 ACK 与 gameplay 事件摘要映射，在 viewer 中统一渲染微循环反馈（toast + 事件行 + HUD 计时/诊断）。
- **Success Criteria**:
- SC-1: 有效控制命中率（可推进控制次数 / 预期推进控制次数）`>= 90%`。
- SC-2: 可玩性卡片题 15/16/17 平均评分 `>= 4.0/5`。
- SC-3: gameplay 关键动作的可见反馈延迟 `P95 <= 1s`（ActionAccepted 或对应 DomainEvent 进入 UI）。
- SC-4: 玩家模式下首屏主面板信息块数量相较 ROUND-002 减少 `>= 25%`（默认状态）。
- SC-5: 手动截图评估中“控制结果可见性”和“世界目标可读性”两项均达到 `Pass`。

## 2. User Experience & Functionality

- **User Personas**:
  - 主用户: 玩家/评测者，需要在 5~15 分钟内看到明确的动作反馈与倒计时。
  - 次用户: 玩法开发者，需要稳定的“动作 -> 事件 -> UI”可观测链路用于回归。
  - 次用户: 发行评审者，需要量化指标与证据包判断微循环是否达标。
- **User Scenarios & Frequency**:
  - 微循环体验（每次会话都发生）：下达动作后需在 1s 内看到“已接受/预计结算”。
  - 版本回归（每个候选版本至少一次）：对照指标与卡片评分验证。
  - 问题诊断（缺陷复盘时）：定位动作失败原因与无进展窗口。
- **User Stories**:
- PRD-GAME-004-01: As a 玩家/评测者, I want immediate action feedback and timers, so that I trust my control.
- PRD-GAME-004-02: As a 玩法开发者, I want runtime to emit actionable ACK + gameplay summaries, so that I can debug loops.
- PRD-GAME-004-03: As a 发行评审者, I want measurable micro-loop metrics, so that release readiness is objective.
- PRD-GAME-004-04: As a 玩家, I want player-mode panel to show only the current focus by default, so that I can decide next action quickly.
- PRD-GAME-004-05: As a 玩家, I want selected targets and map markers to be easy to spot, so that world interaction feels readable.
- **Critical User Flows**:
  1. Flow-MLF-001: `玩家提交动作 -> Runtime 产生 ActionAccepted -> Viewer 显示 ACK + ETA -> 对应 DomainEvent 到达后更新状态`
  2. Flow-MLF-002: `Gameplay DomainEvent 触发 -> 映射为可读摘要 -> Toast/事件行更新 -> HUD 倒计时刷新`
  3. Flow-MLF-003: `连接正常但无进展 -> 诊断最近动作/拒绝原因/资源缺口 -> 输出可执行建议`
  4. Flow-MLF-004: `玩家进入 player 模式 -> 默认 Mission 视图 -> 按需展开模块开关/切换布局 -> 快速进入控制动作`
  5. Flow-MLF-005: `玩家选择目标/缩放 2D 地图 -> 选中强调与 marker 保持可读 -> 执行动作并观察世界反馈`
- **Functional Specification Matrix**:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 动作 ACK 事件 | `action_id/action_kind/actor_id/eta_ticks/notes` | 动作通过校验后立即发出 ACK | `submitted -> accepted -> resolved/failed` | 按事件时间排序 | 系统生成，客户端只读 |
| Gameplay 事件摘要 | `domain_kind/summary/key_ids/tone` | DomainEvent 到达后生成摘要 | `raw -> summarized` | 规则化模板映射 | 系统生成，客户端只读 |
| 微循环 HUD 计时 | `pending_eta_ticks/due_timers` | 每 tick 刷新倒计时 | `pending -> due -> cleared` | 最近到期优先 | 客户端显示 |
| 无进展诊断 | `idle_secs/last_action/reject_reason/suggestion` | 无进展 >= 5s 触发提示 | `idle -> hinted -> recovered` | 只保留最新 | 客户端显示 |
| 可执行动作推荐 | `available_actions/action_label/blocker/recommended_next_step` | summary 刷新或玩家完成动作后更新 1-3 个推荐行动 | `unknown -> actionable/blocked -> resolved` | 当前任务/资源 blocker 优先，避免纯 debug 字段外露 | 客户端显示 |
| 控制结果显著条 | `stage/action/delta/effect` | 控制反馈到达后在 HUD 顶部高亮展示 | `hidden -> highlighted -> steady` | stage 颜色优先于文本细节 | 客户端显示 |
| 玩家模式渐进披露 | `active_preset/module_visibility` | 默认 Mission 预设，模块开关按需展开 | `uninitialized -> mission_default -> user_customized` | 任务主线优先，次级模块后置 | 客户端显示 |
| 世界可读性增强 | `halo_radius/marker_visibility/marker_scale` | 选择目标或缩放地图时动态调整强调 | `normal -> emphasized` | 选中目标优先，2D 全局标记可见 | 客户端显示 |
- **Acceptance Criteria**:
  - AC-1: Runtime 产生 ActionAccepted 且 viewer 可见，形成“已接受 + ETA”反馈。
  - AC-2: Gameplay DomainEvent 均有可读摘要与 tone 分类。
  - AC-3: HUD 展示至少战争/治理/危机/合约倒计时，并在到期时明确提示。
  - AC-4: 无进展提示包含“原因 + 下一步建议”，不再出现纯“等待/重试”。
- AC-5: SC-1/SC-2/SC-3 在 playability 测试中达标。
- AC-6: 完成标准为 required tier 通过 + playability 卡片与指标复核通过。
- AC-7: 控制结果显著条在 `completed_advanced/applied/blocked/completed_no_progress` 阶段均可读且色彩区分明确。
- AC-8: 玩家模式默认布局下模块开关不再常驻展开，且不影响恢复/重试链路可用性。
- AC-9: 2D 场景下 marker 与选中强调在手动截图中可被稳定辨识。
- AC-10: 玩家模式默认不再自动连续推进；手动实测静置 `1s` 时 `delta_tick=0`，并可通过单步操作获得可控反馈。
- AC-11: runtime_live 播放态手动实测稳态推进速率 `delta_tick<=2/s`，暂停态手动实测 `delta_tick=0/s`（同轮截图与测量数据归档）。
- **Non-Goals**:
  - 不改动战争/治理/危机/经济规则本体。
  - 不新增玩法 UI 面板与复杂视觉动效。
  - 不引入新的跨节点协议与共识流程。

## 3. AI System Requirements (If Applicable)
- **Tool Requirements**: 不适用（本 PRD 不改动 LLM 决策或工具链）。
- **Evaluation Strategy**: 通过 playability 卡片 + 行为指标回归验证（见第 6 章）。

## 4. Technical Specifications

- **Architecture Overview**:
  - Runtime 动作校验 -> `ActionAccepted` 事件 -> runtime_live 映射 -> viewer toast/HUD。
  - Gameplay DomainEvent -> 映射摘要 -> 事件行/Toast/倒计时刷新。
  - 无进展监控：基于 tick/event 增量与最近动作状态生成诊断提示。
- **Integration Points**:
  - Runtime 事件与状态：`crates/oasis7/src/runtime/events.rs`、`crates/oasis7/src/runtime/state/apply_domain_event_gameplay.rs`
  - Viewer 映射与反馈：`crates/oasis7/src/viewer/runtime_live/mapping.rs`、`crates/oasis7_viewer/src/egui_right_panel_player_experience.rs`
  - 玩法设计基线：`doc/game/gameplay/gameplay-top-level-design.prd.md`
  - 可玩性卡片：`doc/playability_test_result/playability_test_card.md`
  - 测试门禁：`testing-manual.md`
- **Edge Cases & Error Handling**:
  - 空数据: 无 action/event 时 HUD 显示“暂无行动/事件”，不出现误导性倒计时。
  - ACK 事件缺失：viewer 降级为“等待 DomainEvent”并提示延迟原因。
  - ACK 超时: 超过 eta_ticks 未见 DomainEvent 时提示“仍在处理中”并引导诊断。
  - `available_actions` 为空但 gameplay summary 非空: 不展示普通空状态；必须显式说明“当前无可执行行动”的原因，优先标记 `runtime_snapshot_empty_entities`、资源不足、权限不足或等待推进等 blocker。
  - `available_actions` 存在但无法直接执行: 允许显示为 blocked recommendation，但必须给玩家一个可恢复下一步，例如 claim starter OC、选择目标、推进 tick 或切换到可执行入口。
  - DomainEvent 字段缺失：渲染为通用摘要并记录诊断日志。
  - 事件乱序/重复：按 event_id 去重并以时间戳排序。
  - 并发冲突: 同 tick 多事件按 event_id 排序，保持可复现。
  - Viewer 断线或 tick 停滞：提示“连接/推进异常”，不误导为“操作失败”。
  - 旧版本兼容：不支持 ACK 的 runtime 仍可运行，viewer 提示“协议不支持 ACK”。
  - 数据异常：summary 生成失败时回退为原始事件类型字符串。
  - 权限不足: ACK 与摘要仅由系统生成，客户端写入请求拒绝并提示。
  - 数据迁移: 本迭代不引入数据迁移，旧快照保持兼容。
- **Non-Functional Requirements**:
  - NFR-MLF-1: ACK/DomainEvent 到 UI 可见反馈延迟 `P95 <= 1s`。
  - NFR-MLF-2: 新增映射逻辑对 viewer 帧耗时影响 `P95 <= 2ms`。
  - NFR-MLF-3: 事件摘要与提示支持中英文（`zh/en`）。
  - NFR-MLF-4: 兼容旧 runtime/event schema（无 ACK 也可运行）。
- NFR-MLF-5: 数据规模 10k 事件内，事件行检索不出现可感知卡顿。
- NFR-MLF-6: 新增 DomainEvent 类型时可扩展摘要模板，不要求改 UI 布局。
- NFR-MLF-7: 玩家模式默认布局切换操作额外开销 `P95 <= 10ms`（单帧 UI 预算内）。
- NFR-MLF-8: 可读性增强改动不引入渲染超预算告警（`RenderPerfSummary::auto_degrade_active` 保持稳定）。
- **Security & Privacy**: 不新增敏感字段与玩家私密数据；日志仅记录 action_id 与公开事件信息。

## 5. Risks & Roadmap

- **Phased Rollout**:
  - MVP: ACK 事件 + gameplay 事件摘要 + toast 显示。
  - v1.1: HUD 倒计时 + 无进展诊断建议。
  - v2.0: 指标看板与门禁自动化回归。
- **Technical Risks**:
  - Runtime ACK 与 DomainEvent 口径不一致，导致“确认已接受但后续无结果”的二次困惑。
  - 事件摘要规则过于通用，无法解释关键因果。

## 6. Validation & Decision Record

- **Test Plan & Traceability**:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-001/002 | `test_tier_required` | runtime 事件映射与 viewer toast 单测 | 动作 ACK/事件可见性 |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-003 | `test_tier_required` | HUD 倒计时与无进展诊断回归 | 微循环节奏与控制感 |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-004 | `test_tier_required` + `test_tier_full` | playability 卡片与指标复核 | 发行门禁体验 |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-005 | `test_tier_required` | mission HUD 控制结果显著条与阶段文案回归 | 控制结果可见性 |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-006 | `test_tier_required` | 玩家模式布局默认策略 + 可见性持久化解析回归 | 面板认知负载 |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-007 | `test_tier_required` | 选中强调/2D marker 可见性与缩放策略回归 | 世界可读性 |
| PRD-GAME-004 | TASK-GAMEPLAY-MLF-008 | `test_tier_required` | 手动实操 + 迭代截图视觉评估 | 体验验收证据 |
- **Decision Log**:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-MLF-001 | 新增 ActionAccepted 事件 | 仅依赖后续 DomainEvent 推断 | 避免“已接受但无反馈”的空窗期 |
| DEC-MLF-002 | Gameplay 事件摘要规则化 | 直接输出 Debug `{:?}` | 反馈可读性与控制感提升 |
| DEC-MLF-003 | HUD 倒计时展示 | 仅通过日志/事件行观察 | 微循环节奏需要即时可见指标 |
| DEC-MLF-004 | 在 Mission HUD 顶部增加控制结果显著条 | 仅保留原控制反馈详情卡 | 更快建立“动作已生效”的因果感知 |
| DEC-MLF-005 | 玩家模式默认 Mission 预设 + 模块开关折叠 | 保持所有模块开关常驻展开 | 降低首屏噪声，缩短下一步决策时间 |
| DEC-MLF-006 | 提升选中强调与 2D marker 可读性阈值 | 维持现有最小可见尺寸与可见门槛 | 解决场景空旷下目标辨识困难 |
| DEC-MLF-007 | 玩家模式改为“默认停驻 + 单步优先”控制节奏 | 默认自动连续播放 | 降低 tick 快速跳变导致的视觉闪烁，提升可控性与可读性 |
| DEC-MLF-008 | runtime_live 播放链路引入固定间隔节流（默认 `800ms`） | 读取循环每 `50ms` 推进一步 | 将播放节奏从双位数 tick/s 降至约 `1 tick/s`，显著降低闪烁 |
| DEC-MLF-009 | 将 canonical `available_actions` 作为玩家下一步动机的正式输入，而不是只作为内部诊断字段 | 只在 debug / raw snapshot 中保留 `available_actions` | 2026-07-08 gameplay slice 指出玩家会从“选择行动 -> 反馈 -> 继续决策”退化为“观察世界/猜命令”；推荐行动和 blocker 文案是 micro-loop 的一部分 |

审计轮次: 4
