# LLM Agent Decision Provider 标准层 + Local Provider 外部适配可行性（2026-03-12）

- 对应设计文档: `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前 world-simulator 的 Agent 能力已经具备 `Observation -> AgentBehavior::decide -> AgentDecision -> runtime 校验/执行 -> DecisionTrace` 的完整闭环，但“世界内 Agent 契约”与“具体 LLM / agent 框架实现”仍高度耦合。若未来要评估 `Local Provider` 这类外部 agent framework 参与游戏模拟，缺少一个稳定的中间标准层，会导致接入成本高、风险边界不清、验证口径分散。
- Proposed Solution: 在 world-simulator 的 Agent 层与具体 provider 之间新增 `Decision Provider` 标准层，冻结 `ObservationEnvelope / DecisionRequest / DecisionResponse / FeedbackEnvelope / TraceEnvelope` 这组内部契约；`Local Provider` 通过 adapter 作为可插拔外部 provider 接入，仅负责“决策建议与工具编排”，不替代 runtime 权威规则执行。先完成可行性建模、接口冻结、风险边界与 PoC 路线设计，再决定是否进入实现阶段。
- Success Criteria:
  - SC-1: 明确 `runtime authoritative / provider advisory` 边界，任何外部 provider 都不得直接修改世界状态或绕过 kernel 校验。
  - SC-2: 形成一套 provider-agnostic 的决策标准层契约，能够同时兼容现有本地 `LlmAgentBehavior` 与未来 `Local ProviderAdapter`。
  - SC-3: 定义 `Local Provider` 可行性评估矩阵，覆盖动作映射、工具编排、记忆策略、错误语义、时延/成本与可观测性。
  - SC-4: 形成阶段化落地路线：`MockProvider -> Local Provider PoC -> 低频 NPC 试点 -> 扩面评估`。
  - SC-5: 外部 provider 的非法输出、超时、格式错或 schema 漂移均有统一失败策略，并可映射到 `Wait` 或 `ActionRejected`。
  - SC-6: provider 输出可回写为 `AgentDecisionTrace`，保持 viewer / QA / replay 诊断链路连续。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要判断“外部 agent 框架能否成为世界内 Agent 决策层”的产品与规则边界。
  - `agent_engineer`：需要一个可替换 provider 的抽象层，避免把特定 LLM 框架写死在模拟核心里。
  - `runtime_engineer`：需要保证外部 provider 不破坏 tick 语义、规则校验与回放证据。
  - `qa_engineer`：需要对不同 provider 共用一套 fixture、错误签名与成功判定标准。
- User Scenarios & Frequency:
  - 架构评审：新 agent 框架进入评估名单时，按专题文档审查一次。
  - 接入 PoC：每个外部 provider 至少先完成 mock fixture 与低频 NPC 试点。
  - 发布前回归：只有通过标准层契约与故障策略验证的 provider 才可进入更大规模模拟。
- User Stories:
  - PRD-WORLD_SIMULATOR-036: As an `agent_engineer`, I want a provider-agnostic decision layer between world-simulator and external agent frameworks such as `Local Provider`, so that we can evaluate or swap external agent runtimes without breaking runtime authority, traceability, or QA contracts.
- Critical User Flows:
  1. Flow-DP-001（标准层决策闭环）:
     `World Observation -> DecisionProvider.decide -> candidate action / wait -> runtime validate/execute -> feedback -> trace`。
  2. Flow-DP-002（Local Provider 外部接入 PoC）:
     `ObservationEnvelope + action whitelist -> Local ProviderAdapter -> Local Provider tool/session loop -> structured decision -> Action mapping`。
  3. Flow-DP-003（失败与降级）:
     `provider timeout / malformed output / unknown tool -> adapter classify -> Wait or ActionRejected -> DecisionTrace.error`。
  4. Flow-DP-004（回归验证）:
     `golden observation fixtures -> local provider / Local Provider provider -> compare validity, latency, trace completeness`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 决策标准层 | `ObservationEnvelope/DecisionRequest/DecisionResponse` | provider 仅返回结构化决策建议 | `observed -> decided -> validated -> executed/rejected` | 每个 agent 每 tick 至多产出 1 个决策 | provider 无权直接写 world state |
| 动作白名单 | `ActionCatalog/ActionSchemaRef` | provider 仅能从白名单动作中选择 | `catalog_frozen -> proposed -> validated` | 动作参数需通过 schema 校验 | runtime 最终裁定 |
| 反馈回写 | `FeedbackEnvelope` | runtime 执行结果回写给 provider | `executed/rejected -> feedback_pushed` | 保持 action_id / event 顺序 | 仅 owner agent 可读自身反馈 |
| 可观测性桥接 | `TraceEnvelope -> AgentDecisionTrace` | provider trace 映射到现有 viewer/QA 诊断面 | `trace_pending -> trace_emitted` | trace 与 world time 对齐 | 不得泄露敏感凭据 |
| 记忆同步策略 | `MemoryContextSnapshot/MemoryWriteIntent` | provider 只读注入上下文；写回需经本地策略裁决 | `memory_read -> decide -> optional_memory_write` | 本地 memory 为权威来源 | 外部 memory 为可选缓存 |
| Provider 评估 | `valid_action_rate/timeout_rate/p95_latency/trace_completeness` | 使用固定 fixture 对比 provider 表现 | `bench_pending -> bench_done` | 按场景/agent 类型分层统计 | QA 可复核 |
- Acceptance Criteria:
  - AC-1: 建立 `Decision Provider` 标准层专题文档，明确数据契约、边界、风险与验证口径。
  - AC-2: 明确 `Local Provider` 的角色定位为“外部 provider / adapter”，而非 runtime / kernel 替代物。
  - AC-3: 文档中定义至少一条可执行 PoC 路线，要求先通过 `MockProvider` 与 fixture 验证，再进入 `Local Provider` 试点。
  - AC-4: 文档中冻结最小动作映射策略：只允许先在低频、低破坏性动作集上试点（例如 `wait`、`move`、`chat`、有限查询）。
  - AC-5: 文档中定义统一失败策略与 trace 回写规范，保持与 `AgentDecisionTrace`、`ActionRejected`、viewer 调试面一致。
  - AC-6: 文档中定义 required/full 验证矩阵，并可追溯到本专题项目任务。
- Non-Goals:
  - 不在本轮直接把 `Local Provider` 接入主线模拟代码。
  - 不在本轮重写现有 `LlmAgentBehavior`、memory 系统或 runtime kernel。
  - 不在本轮把 `Moltbook` 等社交层能力引入 world-simulator。
  - 不在本轮把外部 provider 用于高频战斗/经济核心 actor。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - provider 必须支持结构化输入/输出，而不是仅依赖自由文本解析。
  - provider 必须能够暴露或适配“受限工具/动作白名单”，避免任意外部副作用。
  - provider 最好支持会话级 trace、tool trace、latency 指标与 message transcript 导出，便于映射为 `AgentDecisionTrace`。
- Evaluation Strategy:
  - `test_tier_required`: 文档冻结 + golden fixtures + mock provider 验证 + error policy 审查。
  - `test_tier_full`: 引入真实 `Local ProviderAdapter` 后，对低频 NPC 场景执行多轮闭环，比较有效动作率、超时率、trace 完整度与单步成本。
- Model / Provider Requirements:
  - 标准层不得假设固定模型供应商。
  - `Local Provider` 若进入 PoC，仅作为 provider runtime，模型选择、插件、memory backend 均视为 adapter 配置，不得反向污染 world-simulator 核心接口。

## 4. Technical Specifications
- Architecture Overview:
  - `WorldKernel / runtime`: 继续作为唯一权威执行层，负责规则校验、资源消耗、状态变更、事件与 receipt。
  - `AgentBehavior Facade`: 继续对 simulator 暴露 `decide/on_event/on_action_result/take_decision_trace` 语义。
  - `DecisionProvider` 标准层：新增 provider-agnostic 接口，负责把 world observation 转换为 provider request，并接收结构化决策结果。
  - `ProviderAdapter`: 为不同 provider 提供具体适配实现；`Local ProviderAdapter` 作为其中之一。
  - `Trace & Memory Bridge`: 把 provider trace / memory 意图映射回本地 `AgentDecisionTrace` 与 `AgentMemory`。
- Integration Points:
- `crates/oasis7/src/simulator/agent.rs`
- `crates/oasis7/src/simulator/memory.rs`
- `crates/oasis7_proto/src/viewer.rs`
  - `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.prd.md`
- Standard Contracts:
  - `ObservationEnvelope`: `agent_id`、`world_time`、局部可见世界状态、近期事件摘要、记忆摘要、预算与动作白名单。
  - `DecisionRequest`: `observation + action_catalog + provider_config_ref + timeout_budget`。
  - `DecisionResponse`: `decision(wait/act) + action_ref + args + provider_error? + diagnostics + trace_payload`。
  - `FeedbackEnvelope`: `action_id + success/failure + reject_reason + emitted_events + world_delta_summary`。
  - `TraceEnvelope`: provider transcript、tool trace、latency、token/cost、schema repair 记录。
- Local Provider Adapter Strategy:
  - 使用 adapter 把 `ObservationEnvelope` 转成 `Local Provider` 可消费的会话输入。
  - 通过有限 `tool`/`action` 暴露 world 可执行动作，而不是开放任意外部命令。
  - adapter 负责把 `Local Provider` 的结果收敛为 `DecisionResponse`，并将 provider trace 映射到本地 trace 结构。
  - 首期 PoC 仅允许低频动作集，禁止直接接管高频 tick actor 与强一致经济关键路径。
- Edge Cases & Error Handling:
  - provider 超时：返回 `Wait`，同时记录 `llm_error=provider_timeout`。
  - provider 输出未知动作：映射为 `ActionRejected`，写入稳定 `error_code`。
  - provider 工具调用次数超限：adapter 终止会话并输出结构化错误。
  - memory 写回冲突：以本地 memory 为准，provider 仅提交 write intent。
  - 网络故障 / API 401 / 插件不可用：按 provider_error 分类，不影响 runtime 主循环稳定性。
- Non-Functional Requirements:
  - NFR-1: 标准层接口必须支持无网络 mock 执行，确保 required 测试不依赖外部服务。
  - NFR-2: `DecisionResponse` 字段必须稳定可版本化，新增字段只追加，不破坏既有语义。
  - NFR-3: provider trace 必须可裁剪、可脱敏，并可映射到当前 viewer 诊断面。
  - NFR-4: 首期 `Local Provider` PoC 仅允许在低频 agent 上运行；单次决策 p95 目标不高于 3s。
  - NFR-5: required 测试基线必须能在本地无外部 provider 情况下完整执行。
- Security & Privacy:
  - provider 不得持有 world runtime 的直接写权限或底层存储权限。
  - trace / transcript 不得写出私钥、完整 auth proof 或敏感 token。
  - adapter 仅允许访问显式声明的 provider endpoint / plugin 能力，不得隐式穿透宿主环境。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1 (2026-03-12): 完成 `Decision Provider` 标准层 + `Local Provider` 外部适配可行性建模。
  - M2: 落地 provider contract 类型定义与 golden fixture 样本。
  - M3: 实现 `MockProvider`，验证标准层不依赖外部网络即可跑通。
  - M4: 实现 `Local ProviderAdapter` PoC，并在单一低频 NPC 场景试点。
  - M5: 依据 QoS/成本/稳定性结果，决定是否进入多 agent 扩面。
- Technical Risks:
  - 风险-1: `Local Provider` 的会话/工具模型与 tick 驱动 world actor 语义不完全同构，可能导致抽象层过宽或过厚。
  - 风险-2: 外部 provider latency/cost 抬高后，模拟规模与回归频率会受限。
  - 风险-3: provider memory 与本地 `AgentMemory` 双写可能造成上下文漂移。
  - 风险-4: tool schema 演进过快会让 adapter 成为新的维护热点。
  - 风险-5: 若没有 fixture 与统一错误签名，多个 provider 的评估将无法横向比较。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-036 | TASK-WORLD_SIMULATOR-112 | `test_tier_required` | `./scripts/doc-governance-check.sh` | 模块文档入口、专题索引、owner/边界定义 |
| PRD-WORLD_SIMULATOR-036 | T1/T2/T3/T4/T5 | `test_tier_required` / `test_tier_full` | fixture contract test + mock provider + future adapter PoC tests | agent provider abstraction、外部 provider 接入边界、trace 与 memory 桥接 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-DP-001 | 以 `Decision Provider` 标准层隔离 world-simulator 与外部 agent framework | 直接让 `Local Provider` 替代现有 Agent 层 | 当前 runtime / trace / memory 契约已存在，直接替换风险过高且难以验证边界。 |
| DEC-DP-002 | 将 `Local Provider` 定位为 adapter/provider，而非 kernel/runtime 替代物 | 让 `Local Provider` 直接控制世界状态或规则执行 | runtime 权威与 replay/QA 契约必须保留在本地内核。 |
| DEC-DP-003 | 首期仅开放低频、低破坏性动作集做 PoC | 首轮就接入高频战斗/经济关键 agent | 先验证抽象层与 error policy，再决定是否扩面，能显著降低产品与工程风险。 |
