# 纯 API 客户端等价玩法专题 PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.project.md`
- 上游总契约: `doc/core/player-access-mode-contract-2026-03-19.prd.md`

审计轮次: 1

## 0. 设计摘要

### 0.1 目标

- 将“纯 API 客户端”从调试/探针通道提升为正式玩家入口，而不是只能验证协议是否活着。
- 按 `PRD-CORE-009`，本专题承接的正式玩家入口名为 `pure_api`，不与 OpenClaw `headless_agent` execution lane 混用。
- 明确 `pure_api` 的 formal gameplay 与 headed Web/UI 一样要求 active LLM access；无 LLM 时只能保留 observer/debug，不再构成正式可玩或 parity 结论。
- 保证纯 API 客户端与 Web/UI 客户端共用同一套世界状态、可执行动作、阶段目标和持续游玩能力。
- 允许展示形式不同，但不允许信息粒度降级或能力缺失，确保玩家可仅通过 API 长期游玩。

### 0.2 范围

#### In Scope
- 定义纯 API 客户端与现有客户端的玩法等价边界。
- 将当前 Viewer 内组装的关键玩家语义下沉为协议级可读字段。
- 定义纯 API 客户端必须支持的查询、动作、阶段承接与恢复能力。
- 定义 `test_tier_required` / `test_tier_full` 的纯 API 长玩与等价验证口径。

#### Out of Scope
- 不要求纯 API 客户端复刻现有视觉布局、动画、字体或画布表现。
- 不在本期重做全部 Viewer UI 逻辑；仅下沉必须共享的玩法语义。
- 不在本期引入新的玩法分支或扩张新的世界规则。

### 0.3 接口 / 数据

- 上游输入:
  - live viewer 协议快照、事件流、控制反馈、玩家鉴权与聊天接口
  - `FirstSessionLoop` / `PostOnboarding` / MidLoop 阶段机相关世界状态
  - 工业、治理、冲突、危机、交易与恢复相关 runtime 事实状态
- 下游输出:
  - 协议级 `player_gameplay_snapshot`
  - 协议级 `available_actions` / `stage_goal_snapshot` / `next_step_hints`
  - 纯 API 客户端可消费的连续游玩契约
  - 纯 API required/full 验证脚本、证据与 playability 结论

### 0.4 里程碑

- M0 (2026-03-19): 冻结纯 API 等价目标、专题 PRD / design / project 与根入口追踪。
- M1: 下沉 `PostOnboarding`、工业引导和控制反馈所需的最小玩法语义到协议层。
- M2: 提供可持续游玩的纯 API 动作面与恢复面，覆盖从首局到中循环入口。
- M3: 建立纯 API / UI 等价矩阵与长玩 required-tier / full-tier 回归。

### 0.5 风险

- 如果继续把关键玩法语义只留在 Viewer 端，纯 API 客户端永远只能做“世界探针”，不能做正式入口。
- 如果协议层暴露的只是事件原文而没有玩家可消费的聚合语义，API 客户端会被迫重复实现 Viewer 逻辑，形成多份事实源。
- 如果只做首局 smoke，不做持续游玩验证，纯 API 客户端仍可能在中循环前断层。

## 1. Executive Summary

- Problem Statement: 当前无 UI / 协议链路虽已具备正式动作面，但 `pure_api` 是否要求 active LLM access 的口径曾在代码、脚本与文档间分裂，导致“能连协议”“无 LLM 也能正式玩”“active LLM 下的 parity”三者被混写。
- Proposed Solution: 新增纯 API 等价专题，把阶段目标、进度、阻塞、下一步建议和可执行动作下沉到协议级统一快照中，并把 active LLM access 固定为 formal gameplay 前置；无 LLM 路径只保留 observer/debug 结论。
- Success Criteria:
  - SC-1: 纯 API 客户端在 `FirstSessionLoop -> PostOnboarding -> MidLoop entry` 路径上，必须能获得与 UI 客户端同源的阶段目标、进度、阻塞和下一步建议。
  - SC-2: 纯 API 客户端必须支持玩家持续游玩所需的核心动作集合，覆盖观察、选择、推进、聊天/命令、恢复与阶段承接，不允许只能 `step + snapshot`。
  - SC-3: 关键玩法语义 100% 由协议层提供 canonical 字段，不允许 UI 独占“主目标 / blocker / next_step / available_actions”。
  - SC-4: `test_tier_required` 至少具备 1 条纯 API 长玩回归，验证玩家可不依赖浏览器从首局推进到首个持续能力里程碑。
  - SC-5: 纯 API 与 Web/UI 的等价矩阵必须可审计，字段缺失、动作缺失或阶段断层任一出现都判定为阻断。
  - SC-6: 当 LLM 不可用时，`step / play / gameplay_action / agent_chat / prompt_control` 必须返回明确 `llm_mode_required` 或 `llm_init_failed`，并把当前会话标记为 gameplay blocked，而不是继续保留 `parity_verified`。

## 2. User Experience & Functionality

- User Personas:
  - 纯 API 玩家: 希望只用 CLI / agent / API 客户端长期游玩，而不是被迫打开浏览器。
  - 自动化代理/研究者: 需要稳定、完整、无 UI 依赖的玩法接口来驱动长期实验或自动玩家。
  - `viewer_engineer` / `runtime_engineer`: 需要明确哪些语义应属于协议层，哪些仅属于表现层。
  - `qa_engineer`: 需要明确纯 API 何时算“等价可玩”，而不是只算“协议可连”。
- User Scenarios & Frequency:
  - 无浏览器游玩: 高频，适用于自动代理、远端环境或资源受限环境。
  - 持续恢复: 每次重连或换客户端时发生，用于恢复阶段目标和可执行下一步。
  - 版本回归: 每个候选版本至少 1 次，用于验证纯 API 路径未退化为探针模式。
- User Stories:
  - PRD-GAME-008: As a 纯 API 玩家, I want the same gameplay information and actions as the UI client, so that I can keep playing without a browser.
  - PRD-GAME-008-A: As an 自动化代理, I want canonical stage/goal/action fields in the protocol, so that I do not have to re-implement Viewer-only logic.
  - PRD-GAME-008-B: As a 玩法设计者, I want UI and API to share one source of truth for player-facing gameplay state, so that content parity stays enforceable.
  - PRD-GAME-008-C: As a QA, I want a parity matrix and pure API long-play regression, so that “能连协议”与“能玩游戏”被严格区分。
- Critical User Flows:
  1. Flow-API-001: `客户端连接 live 协议 -> 获取 gameplay snapshot -> 看到当前阶段、主目标、进度、阻塞、下一步建议`
  2. Flow-API-002: `客户端列出当前可执行动作 -> 玩家选择动作/聊天/推进 -> 协议返回控制反馈与新的 gameplay snapshot`
  3. Flow-API-003: `玩家完成 FirstSessionLoop -> 纯 API 客户端收到 PostOnboarding 阶段切换与主目标更新 -> 继续推进到首个持续能力里程碑`
  4. Flow-API-004: `玩家被阻塞 -> 协议返回 blocker 类型、影响对象、建议修复动作 -> 玩家按 API 提示恢复后继续游玩`
  5. Flow-API-005: `玩家断线/切换客户端 -> 重新连接 -> 恢复当前阶段、主目标、最近控制反馈与可执行动作集合`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 协议级玩家快照 | `stage_id`、`stage_status`、`goal_id`、`goal_title`、`progress_percent`、`blocker_primary`、`next_step_hint`、`available_actions`、`recent_feedback` | 客户端拉取/订阅后直接渲染，不再需要自行猜测 UI 私有语义 | `syncing -> playable -> blocked -> recoverable -> milestone_completed` | 所有玩家语义来自统一协议快照；UI 与 API 不得各算一套 | 任何已连接客户端可读；写操作按玩家鉴权 |
| 可执行动作列表 | `action_id`、`action_kind`、`target_kind`、`target_id`、`label`、`preconditions`、`disabled_reason` | 客户端可发起 `focus/select/control/chat/submit` 等动作；禁用项必须返回原因 | `hidden -> available -> disabled -> consumed` | 先显示当前阶段最相关动作，再显示次级动作；不得依赖 UI 才知道“下一步能做什么” | 需要鉴权的写操作必须显式标记 |
| 阶段切换与恢复 | `stage_transition_reason`、`entered_at`、`resumable_since`、`resume_hint` | 客户端在阶段切换或重连时展示“当前在哪里、接下来做什么” | `introduced -> active -> blocked -> branch_ready` | 阶段状态由 canonical 状态机决定；不同客户端不可分叉 | 所有玩家可读；系统自动生成 |
| 控制反馈与解释 | `request_id`、`effect`、`delta_logical_time`、`delta_event_seq`、`result_reason`、`player_visible_summary` | 玩家执行动作后必须拿到可解释结果，而不是只能看原始事件 | `accepted -> executing -> completed_advanced/completed_timeout/blocked` | 控制反馈按最近一次玩家动作优先展示；超时与阻塞需区分 | 已鉴权客户端可见自己的动作反馈；全局事实仍可通过事件查看 |
| 纯 API 长玩入口 | `client_mode`、`content_surface`、`parity_level`、`supported_loops` | 客户端声明自己是纯 API 模式；系统不得因无 UI 而降级可达玩法内容 | `observer_only -> playable -> parity_verified` | 默认要求至少覆盖新手、PostOnboarding 和中循环入口 | 只读观察模式允许降级；正式玩家模式不允许降级 |

- Acceptance Criteria:
  - AC-1: 纯 API 客户端必须能看到与 UI 客户端同源的 `stage / goal / progress / blocker / next_step` 五类核心字段。
  - AC-2: 关键玩家语义不得只存在于 Viewer 组装层；如果某字段影响“下一步做什么”，则必须有协议级 canonical 表达。
  - AC-3: 纯 API 客户端必须支持玩家持续游玩所需的动作集合，不允许仅剩 `request_snapshot` 与 `step`。
  - AC-4: 断线重连后，纯 API 客户端必须能恢复当前阶段、主目标、最近一次控制反馈和建议下一步。
  - AC-5: `PostOnboarding` 与首个持续能力里程碑在纯 API 路径中必须可达、可解释、可验证。
  - AC-6: 纯 API 客户端允许不同展示形式，但不允许因为“无 UI”而缺少决定玩家下一步行动所需的信息粒度。
  - AC-7: 必须有独立 parity matrix 对账 UI 与 API 的字段、动作和阶段承接；任何一项缺失都不能给出“等价”结论。
  - AC-8: 必须新增 `test_tier_required` 的纯 API 长玩验证；仅有协议 smoke 不构成验收通过。
  - AC-9: 若协议返回原始事件但缺少玩家可消费语义，结论必须记为 `observer_only` 而不是 `playable parity`。
  - AC-10: 若 active LLM access 缺失或初始化失败，纯 API 会话必须显式返回 gameplay blocked 理由；`--no-llm` 不得再被写成正式可玩或 parity 放行路径。
- Non-Goals:
  - 不要求纯 API 客户端复制 3D/2D 视图、镜头控制、视觉特效和 UI 布局。
  - 不要求所有调试字段都暴露给正式玩家客户端；只要求正式玩法所需信息完整。
  - 不在本期定义新的 Agent 脚本 DSL 或外部编排平台协议。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - 协议级快照与事件订阅能力
  - 玩家命令 / 聊天 / 控制提交接口
  - 纯 API 回归与长玩 smoke / required-tier / full-tier 执行脚本
- Evaluation Strategy:
  - 对账 UI / API parity matrix，验证字段完整率、动作完整率、阶段承接完整率
  - 执行纯 API 长玩回归，验证从首局推进到首个持续能力里程碑
  - 抽样检查玩家是否能仅凭 API 信息判断“当前在哪个阶段、为什么卡住、下一步能做什么”

## 4. Technical Specifications

- Architecture Overview:
  - `runtime_engineer` 继续提供事实状态、事件和权限边界。
  - `viewer_engineer` 将当前 UI 私有的玩家语义聚合逻辑下沉为协议级 canonical snapshot，而不是留在单一前端组件里。
  - 纯 API 客户端只负责呈现和交互，不负责自定义一套玩法语义推导器。
  - UI 客户端与纯 API 客户端都消费同一套 `player_gameplay_snapshot + available_actions + control_feedback`。
- Integration Points:
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
  - `crates/oasis7_proto/src/viewer.rs`
  - `crates/oasis7/src/viewer/runtime_live.rs`
  - `crates/oasis7_viewer/src/egui_right_panel_player_guide.rs`
  - `crates/oasis7_viewer/src/web_test_api.rs`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 协议只返回原始事件无聚合语义: 结论必须降级为 `observer_only`，不得宣称“可玩等价”。
  - 某动作在 UI 可做但 API 无入口: 视为 parity 阻断，必须补接口或回退 UI 承诺。
  - 某字段在 UI 可见但 API 不可见: 视为信息粒度缺口，必须补协议字段或取消 UI 独占表达。
  - 重连后阶段丢失: 客户端必须至少恢复 `stage_id / goal_id / blocker / next_step / recent_feedback`，否则判定不可持续游玩。
  - 动作需要鉴权但客户端未 bootstrap: 协议必须返回明确的 `auth required` 和可恢复路径，而不是静默失败。
  - active LLM access 缺失或 provider init 失败: formal gameplay 必须降级为 `blocked`，并返回 `llm_mode_required` / `llm_init_failed` 与配置提示；此路径只能归档为 observer/debug。
  - 世界状态变化过快: 快照与事件必须提供版本/时间基线，避免 API 客户端根据旧快照做错误决策。
  - UI / API 计算结果不一致: 必须以 canonical 协议语义为准，不允许两边各算一套。
- Non-Functional Requirements:
  - NFR-API-1: 100% 关键玩家语义字段在协议层有 canonical 定义，并由 UI / API 共用。
  - NFR-API-2: 纯 API required-tier 长玩回归在本地 fresh bundle 下可复跑，且单次能推进到首个持续能力里程碑。
  - NFR-API-3: 纯 API 客户端恢复当前阶段与主目标的 P95 时间 <= 2 秒（本地 live 会话）。
  - NFR-API-4: parity matrix 覆盖率 100%，字段/动作/阶段三类均需对账。
  - NFR-API-5: 正式玩家 API 模式不得因“无 UI”被自动降级为仅观察模式，除非明确声明 `observer_only`。
  - NFR-API-6: 相关变更 1 个工作日内同步回写 `game` 根 PRD / project / 索引 / testing 入口。
  - NFR-API-7: 100% formal pure API playability / parity 结论都必须基于 active LLM access 路径；no-LLM 仅可输出 blocked 或 observer/debug 结论。
- Security & Privacy:
  - 正式玩家写操作必须继续走现有鉴权边界，不因纯 API 客户端引入越权写入。
  - 协议级玩家语义不得泄露不应暴露给当前玩家的对手私有信息。
  - 纯 API 长玩证据应记录必要审计字段，但不额外采集敏感个人数据。

## 5. Risks & Roadmap

- Phased Rollout:
  - MVP: 协议级玩家快照 + `PostOnboarding` / 工业引导语义下沉 + parity matrix。
  - v1.1: 完整可执行动作列表、重连恢复、纯 API required-tier 长玩脚本。
  - v2.0: 中循环 / 治理 / 冲突方向的纯 API 等价验证与 full-tier 长稳回归。
- Technical Risks:
  - 风险-1: 当前 Viewer 聚合逻辑分散，抽离 canonical 语义时容易出现重复定义或字段漂移。
  - 风险-2: 若先做客户端而不先定协议契约，会把“纯 API 等价”做成第二套前端逻辑。
  - 风险-3: QA 若仍只看 UI 证据，纯 API 退化会长期潜伏。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-008 | `TASK-GAME-023/060` + `TASK-GAMEPLAY-API-001/002/003/004/005` | `test_tier_required` + `test_tier_full` | 文档治理检查、协议字段对账、active-LLM 纯 API required-tier 长玩、UI/API parity matrix、full-tier 长稳抽样与 no-LLM 阻断签名核验 | 新手阶段承接、PostOnboarding、纯 API 客户端、Viewer 协议边界与 formal gameplay 前置条件 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-API-001 | 以“协议级 canonical 玩家语义”作为纯 API 等价基础 | 保持 UI 端继续私有组装语义，API 客户端自行推断 | 多份语义推导器会导致长期漂移，无法做严格 parity。 |
| DEC-API-002 | 允许展示形式不同，但不允许信息粒度和动作能力降级 | 以“无 UI 所以少信息也合理”作为默认前提 | 用户目标是纯 API 正式游玩，不是调试观察。 |
| DEC-API-003 | 把纯 API 长玩纳入 required-tier / full-tier 验收 | 仅保留协议 smoke | smoke 只能证明“活着”，不能证明“能持续玩”。 |
