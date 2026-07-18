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

超时策略说明：当 `[llm].timeout_ms` 配置小于 `30_000` 且首次请求超时，会自动使用 `30_000ms` 进行一次重试，以降低弱网和冷启动导致的误降级概率。

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

### 决策协议（LLM 输出）
- 约定输出 JSON：
  - `{"decision":"wait"}`
  - `{"decision":"wait_ticks","ticks":3}`
  - `{"decision":"move_agent","to":"loc-2"}`
  - `{"decision":"harvest_radiation","max_amount":20}`
- 若输出不合法或字段缺失，回退 `AgentDecision::Wait`。

## 5. Risks & Roadmap
- M1：完成 `LlmAgentConfig` 与 system prompt 默认回退。
- M2：完成 OpenAI 兼容客户端与 `LlmAgentBehavior` 主流程。
- M3：补充测试与文档（README / project / devlog）。

### Technical Risks
- **输出不稳定风险**：LLM 可能输出非 JSON；通过“严格协议 + 解析失败降级”缓解。
- **网络依赖风险**：在线调用可能超时或失败；通过超时配置与 `Wait` 降级缓解。
- **端点与超时配置风险**：端点路径或短超时可能导致请求失败；通过端点规范化（兼容已包含 `/chat/completions` 或 `/responses` 的配置）和超时回退重试缓解。
- **安全风险**：错误 prompt 可能诱导越权动作；当前通过动作白名单与解析协议收敛风险。
- **一致性风险**：未接入 receipt 体系前，跨运行不可严格重放；后续在 runtime effect/receipt 中补齐。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
