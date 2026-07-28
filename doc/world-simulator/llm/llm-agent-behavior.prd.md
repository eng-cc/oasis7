# oasis7 Simulator：LLM 驱动 Agent 行为落地（设计文档）

- 对应设计文档: `doc/world-simulator/llm/llm-agent-behavior.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-agent-behavior.project.md`

审计轮次: 5

## 1. Executive Summary
- 在现有 `AgentBehavior` 抽象上落地一套可运行的 **LLM Agent 行为实现**，用于替代纯规则型 `decide` 逻辑。
- 采用 **OpenAI 兼容 API** 完成推理调用，支持以 `config.toml` 注入关键配置。
- 新增 `system prompt` 配置能力；当未配置时，默认使用：
  - `硅基个体存在的意义是保障硅基文明存续和发展；`
- 保持运行时稳定性：当 LLM 调用失败或输出不可解析时，Agent 应安全降级为 `Wait`，避免破坏模拟闭环。

## 2. User Experience & Functionality

### In Scope
- 新增 `LlmAgentBehavior`，实现 `AgentBehavior` trait。
- 新增 `LlmAgentConfig`（模型、端点、鉴权、超时、system prompt）。
- 新增 system prompt 配置读取与默认值回退。
- 新增基于 `async-openai` 的 Responses API 客户端（同步封装调用）。
- 新增 LLM 输出到 `AgentDecision` 的最小解析协议（`wait / wait_ticks / move_agent / harvest_radiation`）。
- 新增单元测试（配置读取、默认 prompt、决策解析、失败降级）。

### Out of Scope
- 预算与成本计费（token/cost）完整治理链路。
- 记忆模块（Memory Module）WASM 化落地。
- 在 runtime effect/receipt 流中持久化 LLM 输入输出（本次仅在 simulator 侧落地最小闭环）。
- 多模型路由、缓存与重试策略编排。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 配置文件项（`config.toml`）
- 根级选择项：
  - `model` / `model_provider` / `profile`
- Provider 与 Profile：
  - `[model_providers.<name>]`：`base_url` / `auth_token`
  - `[profiles.<name>]`：`model` / `model_provider`
- LLM 行为参数（`[llm]`）：
  - `timeout_ms`（可选，默认 `180000`）
  - `system_prompt`（可选；缺省回退默认 system prompt）
  - `short_term_goal` / `long_term_goal`
  - `max_module_calls` / `max_decision_steps` / `max_repair_rounds`
  - `prompt_max_history_items` / `prompt_profile`
  - `force_replan_after_same_action` / `harvest_max_amount_cap`
  - `execute_until_auto_reenter_ticks` / `debug_mode`

超时策略说明：`timeout_ms` 是单次 provider 请求的明确上限；当前客户端不得在超时后隐式改用更长超时重试。可重试的 provider 错误遵循明确、有限的重试预算并进入 diagnostics；超时、格式错误和预算耗尽都必须留下可解释错误并安全收敛为 `Wait`，不得伪造替代动作。

说明：`config.toml` 使用小写 TOML 字段；环境变量 `OASIS7_LLM_*` 保留为回退与运行时注入通道。

### 配置加载优先级
1. 若项目根目录存在 `config.toml`，优先读取该文件。
2. 若 `config.toml` 不存在，则回退读取进程环境变量。
3. 在读取 `config.toml` 时，若单个键不存在，允许回退到同名环境变量。
4. `model` 按 `[llm].model -> selected profile model -> root model -> ENV -> builtin default` 解析；provider 按 `[llm].model_provider -> selected profile provider -> root provider -> sole provider table` 选择。
5. `base_url` / credential 按 `[llm]` 直接覆盖值优先，再读取 selected provider 的 `base_url` 与 `auth_token`（兼容 provider `api_key`），最后回退 ENV。Agent 目标先读取 `[llm.agent_overrides.<NORMALIZED_AGENT>]`。

现有 `config.toml` 若 TOML 语法错误必须显式失败，不能把“缺键可回退 ENV”误解成“整份坏文件静默回退”。Native launcher Settings 只读写 `[llm].api_key`、`[llm].base_url`、`[llm].model`，保存时必须保留其他表，清空值表示删除对应键；这些直接值会覆盖 profile/provider 的相应解析结果。Web launcher 的 LLM 设置使用浏览器存储，不消费这组文件读写合同。

### 核心结构
- `LlmAgentConfig`
  - `model`, `base_url`, `api_key`, `timeout_ms`, `system_prompt`
- `LlmCompletionClient` trait
  - `complete(request) -> Result<String, LlmClientError>`
- `OpenAiChatCompletionClient`
  - 对接 OpenAI 兼容 `/responses`（通过 `async-openai`）
- `LlmAgentBehavior<C: LlmCompletionClient>`
  - 在 `decide` 中调用 LLM 并解析为 `AgentDecision`

### Demo 入口（已落地）
- `oasis7_llm_agent_demo`
  - 启动后为场景内每个 agent 构造 `LlmAgentBehavior::from_env(agent_id)`。
  - 使用 `AgentRunner` 执行 `observe -> decide -> act` 循环。
  - 支持参数：`--scenario <name>`、`--ticks <n>`；默认场景为 `llm_bootstrap`。
- `oasis7_viewer_live --llm`
  - 在线 viewer server 可切换到 LLM 决策驱动（默认仍为 script）。
  - 启动参数增加 `--llm`；建议配合 `llm_bootstrap` 场景使用。

### Responses provider、工具和错误边界
- 当前 OpenAI-compatible 实现使用 `async-openai` Responses API；`base_url` 接受 API base、`/v1/`、旧 `/v1/chat/completions` 与 `/v1/responses` 后缀并归一到 API base。
- 请求以 `instructions + input + tools` 组装。Responses 的 function call 优先映射为受限内部模块或 `agent_submit_decision`；文本 JSON 仅保留兼容解析路径，不能绕过 schema、当前 Agent 身份或 runtime 校验。
- provider / parser / tool 失败必须区分写入 `llm_error`、`parse_error`、step trace 与 diagnostics；diagnostics 可以记录 model、延迟、prompt/completion/total token 和有限 retry count。它们是观测与成本输入，不等同于计费、发布准入或 runtime receipt。

### 工业调试能力与评测边界
- Agent 只能经 schema 受限的候选决策提出采矿、精炼等已注册动作；动作目录、资源/经济语义、quote、拒绝与 replay 仍由 runtime / gameplay authority 定义和裁定，provider 不获得直接世界状态写权限。
- `agent_debug_grant_resource` 仅在显式 `debug_mode=true` 时加入 provider tool 集；普通模式不得暴露它，模型即使伪造该动作也必须收敛为可解释错误或无状态变化的 `Wait`。带调试注入的运行仅可构造/诊断场景，不能充当正常工业行为、provider parity、成本或稳定性样本。
- 多场景长跑的 trace、`report.json`、`run.log` 与 `summary.txt` 是行为诊断工件。它们能揭示 action、错误、prompt 大小和卡死/回退信号，但单次样本、聚合总量或并行运行本身不证明 provider parity、真实成本稳定性、默认启用资格或玩家体验等价；这些结论仍须使用 `decision-provider-contract` 和 provider parity 专题的固定口径与证据。

### Prompt、记忆和有界决策循环
- 当前 prompt 是 tool-only：每个对话 turn 只允许一个查询工具或最终 `agent_submit_decision`，不得混用查询与最终决策，也不得把自由文本当成动作授权。
- `PromptBudget` 对 observation、conversation/module history 与 memory digest 施加优先级和裁剪；协议/schema 等高优先级段优先保留，软段先压缩。记忆是本地权威上下文，不把外部 provider 缓存当成世界事实。
- 每个 tick 的 module calls、decision turns 和 repair rounds 都有上限。repair 只针对可解释的输出/协议问题；超过上限、无终态决策或不合法动作收敛为 `Wait`，避免卡死或无界上下文膨胀。
- 单个 completion 含多个可解析片段时，当前 guardrail 只选择一个稳定终态（或可用回退）并留下 collapse trace；它不会顺序执行同一 completion 内的多个 module/action 片段。

### 对话、工具和长期记忆可解释性
- 只有 trim 后非空的 `message_to_user` 才写入 `LlmChatRole::Agent`；原始 LLM JSON 绝不伪装为玩家可读 Agent 消息。player、agent、tool、system 是不同 trace roles，module result 保持 tool/system 反馈，不与聊天文本混淆。
- `AgentChat` 仅在 live LLM lane 接受并返回 ack；脚本模式或无效请求返回结构化 error。Web/Viewer 的响应式展示仍由其专业 authority 负责，本专题不退役 EGUI 或声明跨 surface 视觉等价。
- 当长期记忆影响当前行动，玩家侧合同仍要求 `memory_summary`、来源事件/分类、当前用途、纠正提示、置信/过期性与纠正结果。当前实现或 surface 尚未完整证明这些字段、纠正接受范围和最早生效行动；不得把内部 chat history 或 trace 当作已满足的玩家承诺。

### 决策协议（LLM 输出）
- 最终决策经 `agent_submit_decision` 工具提交；其参数可表达 `wait`、`wait_ticks`、移动、采集及已注册动作，并可带可选 `message_to_user`。
- 若输出不合法或字段缺失，回退 `AgentDecision::Wait`。

## 5. Risks & Roadmap
- M1：完成 `LlmAgentConfig` 与 system prompt 默认回退。
- M2：完成 OpenAI 兼容客户端与 `LlmAgentBehavior` 主流程。
- M3：补充测试与文档（README / project / devlog）。

### Technical Risks
- **输出不稳定风险**：LLM 可能输出非 JSON；通过“严格协议 + 解析失败降级”缓解。
- **网络依赖风险**：在线调用可能超时或失败；通过超时配置与 `Wait` 降级缓解。
- **端点与超时配置风险**：端点路径或短超时可能导致请求失败；通过端点规范化、有限且可诊断的重试与 `Wait` 降级缓解，而不是隐藏地延长调用期限。
- **安全风险**：错误 prompt 可能诱导越权动作；当前通过动作白名单与解析协议收敛风险。
- **一致性风险**：未接入 receipt 体系前，跨运行不可严格重放；后续在 runtime effect/receipt 中补齐。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。

## 7. 模块生命周期与市场决策边界

- LLM/provider 可以表达 compile/deploy/install、offer/bid/purchase/delist/destroy/cancel 等模块意图，但 parser/schema 通过不等于动作已接受；所有意图必须进入 runtime 权威校验、共识提交与结果回执。
- production 不允许 runtime source compile。相关决策在 production 返回结构化不可用/外部 builder 恢复提示；仅显式 dev/test 环境可使用受限 source package。
- 决策字段中的 actor 只能解析为当前被授权 Agent；hash、版本、价格、订单标识与必填 payload 必须严格校验，非法输出收敛为结构化拒绝或无状态变化的 `Wait`。
- `module.lifecycle.status` 或等价观测只读取已知 artifact、instance、ownership、listing/order 和 installed 状态；它是派生观测，不是第二 registry，也不能宣称真实激活或保证成交。
- 只有 committed action 的 replay/result 才更新行为回执与长期记忆；空轮询、parser success、submit acceptance 或 viewer snapshot 不得伪造世界进展。

## 8. 历史专题吸收与未收口债务（2026-07-28）

- 本专题吸收五组已退役历史三件套的现行 Agent 语义；它们不再是 authority。
- `llm-prompt-effect-receipt` 保留为 live debt：目前 module-call intent/receipt trace 与 journal 记录不自动证明 action-result 因果、无 provider 重调或无重复执行。其 replay/causality closure 仍由 `decision-provider-contract` 的 pending T4/T5 与 runtime 专业验证负责。
- Local Provider parity、真实 provider 成本/稳定性、完整玩家记忆纠正和跨 surface chat 体验仍未因本次文档合并完成；继续使用 provider parity、LMSO29 稳定性和游戏/Viewer 专题的独立证据。
