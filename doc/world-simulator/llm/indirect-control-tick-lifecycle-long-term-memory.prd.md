# 间接控制链路 + WASM Tick 生命周期 + 长期记忆持久化（设计文档）

- 对应设计文档: `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.project.md`

审计轮次: 5

## 1. Executive Summary
- 收口 README 对“玩家间接控制”的约束：玩家不能直接提交世界动作操控 Agent，只能通过间接链路（提示词/对话）影响 Agent 决策。
- 将 Runtime 模块调用从“仅动作/事件触发”扩展为“支持每 tick 生命周期回调”，并由 WASM 模块自行声明下一次唤醒时机，避免无差别全量调用。
- 将 LLM Agent 的长期记忆从进程内临时态升级为世界快照可持久化状态，支持重启恢复。
- 长期记忆若影响当前 agent 行动，必须提供玩家可读摘要、来源、当前用途和纠正提示；持久化不能变成玩家无法理解或修正的黑箱。

## 2. User Experience & Functionality
- In scope
  - Simulator
    - `ActionEnvelope` 增加提交者身份（system/agent/player）并在 `WorldKernel::step` 前做授权校验。
    - `AgentRunner` 改为以 agent 身份提交动作。
    - Viewer live 的 `prompt_control` / `agent_chat` 链路增加 player-agent 绑定与鉴权。
  - Runtime
    - `ModuleSubscriptionStage` 新增 tick 生命周期阶段。
    - 新增模块 tick 调度状态与模块输出唤醒指令（wake/suspend）。
    - `step_with_modules` 接入 tick 生命周期调度。
  - LLM memory
    - 世界模型新增持久化长期记忆字段。
    - LLM behavior 增加长期记忆导出/恢复接口。
    - Viewer live 在 driver 初始化与每 tick 后完成记忆读写同步。
    - 玩家可读长期记忆合同：当长期记忆被用于当前决策时，surface 至少展示 `memory_summary`、`memory_source_event`、`used_for_current_decision`、`player_correction_hint`、`memory_confidence_or_staleness`。
  - 测试
    - 覆盖间接控制拒绝路径、tick 调度唤醒/挂起语义、长期记忆持久化 roundtrip。
- Out of scope
  - 完整 RBAC/多租户权限系统。
  - 跨节点分布式玩家身份认证协议重构。
  - 长期记忆向量检索或外部数据库接入。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
### 1) Simulator 间接控制链路
- `ActionEnvelope` 新增 `submitter` 字段（默认 system，兼容旧快照）。
- `WorldKernel` 新增按 submitter 的动作授权：
  - `player` 直接 world action 拒绝；
  - `agent` 仅允许提交与自身一致的 agent/owner 相关动作；
  - `system` 保持现状。
- Viewer live：
  - `prompt_control` / `agent_chat` 请求接入 `player_id` 校验。
  - 维护 `agent -> player` 绑定（首次绑定、后续一致性校验）。

### 2) Runtime 每 tick 生命周期调度
- ABI：
  - `ModuleSubscriptionStage` 增加 `Tick`。
  - `ModuleOutput` 增加可选 `tick_lifecycle` 指令（`wake_after_ticks` 或 `suspend`）。
- Runtime：
  - `World` 维护模块 tick 调度表（模块下一次唤醒 tick）。
  - 激活 tick 订阅模块时进行初次调度。
  - 每个 `step_with_modules` 执行 tick 调度：
    - 仅调度到期模块；
    - 调用前先移除当前计划，是否继续调度由模块输出指令决定。

### 3) 长期记忆持久化
- `WorldModel` 新增 `agent_long_term_memories`（`agent_id -> Vec<LongTermMemoryEntry>`，默认空）。
- `LlmAgentBehavior` 增加长期记忆导出/恢复方法。
- 最小玩家可读记忆条：
  - `memory_summary`: 脱敏后的玩家可读摘要，不暴露私有 prompt 全文。
  - `memory_source_event`: 记忆来自哪次玩家消息、系统反馈、工具结果或世界事件。
  - `used_for_current_decision`: 该记忆是否影响当前 agent 行动，以及影响哪一步。
  - `player_correction_hint`: 玩家可如何修正、覆盖或要求忽略该记忆。
  - `memory_confidence_or_staleness`: 记忆是否可能过期、低置信或需要复核。
- Viewer live：
  - 初始化 LLM driver 时，从 `WorldModel` 恢复每个 agent 记忆。
  - 每 tick 执行后，将 runner 中记忆回写到 `WorldModel`，纳入 snapshot/journal 持久化闭环。
  - 若当前行动引用长期记忆，Viewer / API 必须能展示至少 1 条相关记忆摘要与其当前用途；不能只在内部 trace 保留。

## 5. Risks & Roadmap
- M1：文档与任务拆解完成（T0）。
- M2：间接控制链路完成（T1）。
- M3：WASM tick 生命周期与按需调度完成（T2）。
- M4：长期记忆持久化闭环完成（T3）。
- M5：回归、文档状态、devlog 收口（T4）。

### Technical Risks
- 兼容性风险：`ActionEnvelope` / ABI 字段扩展需 `serde(default)`，避免旧快照反序列化失败。
- 行为风险：新增 submitter 授权后，现有调用方若未标注身份可能出现拒绝，需要保留 `system` 默认路径。
- 性能风险：tick 生命周期若调度/移除逻辑不当可能导致重复调用或漏调，需要测试覆盖 wake/suspend/无指令路径。
- 持久化风险：长期记忆回写时机若不稳定，可能造成内存态与快照态漂移，需要在 live tick 闭环中固定同步点。
- 玩法风险：agent 基于错误、过期或玩家已改变主意的记忆继续行动时，如果玩家看不到摘要和纠正入口，会削弱间接控制的 agency 与回流动力。

## 6. Validation & Decision Record
- Acceptance Notes:
  - 若 agent 当前行动引用长期记忆，正式玩家 surface 必须展示记忆摘要、来源和当前用途；缺任一项只能算技术持久化通过，不能算玩家侧 control-feeling 通过。
  - 若玩家发现记忆过期或错误，系统至少需要提供 `player_correction_hint`，说明可通过新消息、重排意图或忽略提示来修正。
  - 系统不得为满足可读性而暴露完整私有 prompt、内部 chain trace 或其他玩家私有记忆。
- Edge Cases:
  - 记忆来源不明：标记为 `memory_source_unknown`，不得强行包装成可靠事实。
  - 记忆过期仍影响行动：标记为 `memory_stale_affects_decision`，必须提示玩家确认或纠正。
  - 玩家纠正后 agent 仍继续按旧记忆行动：标记为 `memory_correction_ignored`，视为 agency weakened。
- Decision Log:
  - DEC-LTM-001: 长期记忆持久化需要玩家可读最小合同，而不是只做 snapshot/journal roundtrip；原因是长期记忆会改变 agent 行动，玩家必须能理解和纠正其影响。
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
