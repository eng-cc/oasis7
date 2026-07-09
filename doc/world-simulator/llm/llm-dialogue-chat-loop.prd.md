# oasis7 Simulator：LLM 对话轮次驱动与右侧 Chat 面板（设计文档）

- 对应设计文档: `doc/world-simulator/llm/llm-dialogue-chat-loop.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-dialogue-chat-loop.project.md`

审计轮次: 5

## 1. Executive Summary
- 将当前 LLM Agent 从“step 概念驱动”收敛为“会话轮次驱动”，避免在 prompt/trace 中暴露 `step_index/step_type` 语义。
- 支持稳定多轮对话：
  - 动作被拒绝后，把拒绝原因作为会话反馈提供给 LLM。
  - 工具调用后，把工具结果作为会话反馈提供给 LLM。
- 在 viewer 右侧新增玩家与 Agent 的 Chat 面板，支持玩家向指定 Agent 发送消息并可视化消息流。
- 保持现有 `oasis7_viewer_live --llm` 与 web 闭环链路可用。
- 对话内容若进入长期记忆并影响后续行动，必须能追溯为玩家消息、系统反馈、工具结果或世界事件，且玩家能看到脱敏摘要与纠正提示。

## 2. User Experience & Functionality

### In Scope
- LLM 决策循环改造为“轮次”模型（assistant 输出 -> 可能 tool_call -> tool_result 回灌 -> assistant 继续）。
- Prompt 组装移除 step 元信息段，增加会话历史段。
- 新增会话消息模型（玩家、Agent、工具、系统反馈）并落到 `AgentDecisionTrace`，用于 viewer 展示。
- 动作执行结果（成功/拒绝）写入会话历史，并在后续轮次可见。
- 会话消息进入长期记忆前需要保留最小来源分类：`player_preference`、`player_instruction`、`world_fact`、`tool_result`、`failure_reason`；后续行动引用该记忆时必须能回指来源。
- viewer 协议新增 Agent Chat 请求/响应。
- viewer 右侧模块新增 Chat 区块（Agent 选择、玩家输入、消息列表）。
- 测试补充：
  - `test_tier_required`：解析、会话回灌、协议序列化、UI 关键逻辑。
  - `test_tier_full`：viewer live + web 闭环回归（保持口径）。

### Out of Scope
- 新模型 provider 切换与成本优化。
- 长期记忆策略重构（只做会话层接入，不做全量记忆系统重写）。
- viewer 视觉风格大改（仅新增功能模块）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) LLM 决策轮次
- 新语义：`max_dialogue_turns`（可兼容旧 `max_decision_steps` 配置读取）。
- 每轮只要求返回一个终态决策 JSON 或一个 `module_call` JSON。
- 不再要求/强调 `plan/decision_draft/final_decision` 分阶段输出。
- 若返回 `module_call`：
  - 执行工具；
  - 将工具结果写入会话（tool role）；
  - 继续下一轮。
- 若返回终态决策：结束本 tick 决策。

### 2) 会话消息模型
- 新增 `LlmChatMessageTrace`：
  - `time`
  - `agent_id`
  - `role`（`player | agent | tool | system`）
  - `content`
- 进入长期记忆的消息或摘要必须额外具备：
  - `memory_source_event`
  - `memory_source_type`
  - `memory_summary`
  - `used_for_current_decision`
  - `player_correction_hint`
- `AgentDecisionTrace` 新增：`llm_chat_messages: Vec<LlmChatMessageTrace>`（记录本次决策新增消息）。
- 行动反馈回灌：
  - `on_action_result` 将成功/拒绝事件转为 `system` 消息。
  - 拒绝时明确携带 `RejectReason` 文本。
  - 系统/工具反馈进入长期记忆时必须区分事实、失败原因和玩家偏好，避免把一次临时失败误写成长期偏好。

### 3) Viewer 协议
- `ViewerRequest` 新增：`AgentChat { request }`
  - `request.agent_id`
  - `request.message`
  - `request.player_id`（可选）
- `ViewerResponse` 新增：
  - `AgentChatAck { ack }`
  - `AgentChatError { error }`
- live server 在 `--llm` 模式下把玩家消息注入对应 Agent 会话；脚本模式返回错误。

### 4) Viewer 右侧 Chat 面板
- 右侧模块开关新增 `chat`。
- Chat 区块能力：
  - 选择目标 Agent。
  - 发送玩家消息（通过 `ViewerRequest::AgentChat`）。
  - 展示玩家/Agent/工具/系统消息流（基于 decision trace 聚合）。

## 5. Risks & Roadmap
- M1：文档与任务拆解完成。
- M2：LLM 会话轮次驱动 + 拒绝/工具反馈回灌完成。
- M3：viewer 协议与右侧 Chat 面板完成。
- M4：测试与文档回写收口。

### Technical Risks
- 会话长度膨胀风险：通过消息条数与字符摘要上限裁剪。
- 协议兼容风险：新增字段保持向后兼容（旧字段保留）。
- UI 交互风险：发送失败需可见错误反馈，避免“静默失败”。
- 玩法风险：玩家不知道哪句话进入了长期记忆时，后续 agent 行动会显得像黑箱；记忆摘要和纠正提示必须优先于完整日志回放。

## 6. Validation & Decision Record
- Acceptance Notes:
  - 玩家消息、工具结果或拒绝原因被长期记忆消费时，后续引用必须能展示来源类型与摘要；不得只在原始 chat history 中要求玩家自行翻找。
  - `memory_source_type` 必须区分玩家偏好、玩家指令、世界事实、工具结果和失败原因，避免 agent 把临时战术或错误反馈长期化。
  - 若引用记忆影响当前行动，surface 必须能说明“这条记忆影响了当前哪一步”。
- Edge Cases:
  - 玩家临时命令被当作长期偏好：标记为 `temporary_instruction_persisted_as_preference`，需要纠正提示。
  - 工具失败被当作世界事实：标记为 `tool_failure_persisted_as_fact`，后续决策不得无解释复用。
- Decision Log:
  - DEC-CHAT-001: Chat loop 的长期记忆输入必须保留来源分类和玩家可读摘要；原因是多轮对话不仅是日志，也会塑造 agent 后续行为。
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
