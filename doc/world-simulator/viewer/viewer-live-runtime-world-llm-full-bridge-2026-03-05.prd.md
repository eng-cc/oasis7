# Viewer Live runtime/world 真 LLM 全量接管（LLM 决策 + 100% 事件/快照 + hard-fail）（2026-03-05）

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.project.md`

审计轮次: 5

## 1. Executive Summary
- Problem Statement: runtime live 仍存在启发式 sidecar 与不完整事件/快照映射，导致“LLM 模式”与实际行为不一致、可观测性不足、回归风险不可控。优先级来源: 用户明确为 P0（真实 LLM 预期不一致）。本专题承接并收口已退役的 `PRD-WORLD_SIMULATOR-016~018` runtime-world migration 阶段目标；当前 `oasis7_viewer_live` 为 runtime/world only，不再使用旧 `--runtime-world` alias。
- Proposed Solution: runtime live 以真实 `AgentRunner + LlmAgentBehavior` 取代启发式 sidecar，建立 runtime->shadow WorldKernel 观测面，扩展 viewer 协议以完整承载 runtime DomainEvent 与 WorldState 快照，LLM 失败统一硬失败并输出可诊断 DecisionTrace。
- Success Criteria:
  - SC-1: runtime live llm 模式下不再调用任何启发式 sidecar 逻辑；所有决策均来自 `AgentRunner<LlmAgentBehavior<_>>`。
  - SC-2: runtime DomainEvent 映射覆盖率 100%，无“未映射/静默丢弃”事件（由单测强制枚举覆盖）。
  - SC-3: runtime WorldState 快照字段覆盖率 100%，viewer 侧可观察到所有运行时状态字段（由覆盖测试强制）。
  - SC-4: LLM 调用失败/超时时硬失败，返回结构化错误与 DecisionTrace，不存在降级启发式动作。
  - SC-5: DecisionTrace 在每次 LLM 决策后输出（含 prompt/profile digest、action 列表、延迟、错误），并可由 viewer 订阅显示。

## 2. User Experience & Functionality
- User Personas:
  - 玩法架构开发者：需要 runtime live 真实 LLM 行为与 runtime 规则一致。
  - LLM 工程师：需要可诊断 DecisionTrace 与硬失败语义，避免隐式降级掩盖问题。
  - 回归测试人员：需要 100% 事件/快照覆盖与可自动验证的回归基线。
- User Scenarios & Frequency:
  - 日常联调：使用 `oasis7_viewer_live --llm` 验证 LLM 决策与 runtime 行为一致性。
  - 发布前回归：执行映射覆盖测试与 DecisionTrace 采证，确保无事件/快照缺口。
  - 故障排查：LLM 失败时通过 DecisionTrace 与结构化错误快速定位。
- User Stories:
  - As a 玩法架构开发者, I want runtime live to use the true LLM decision chain, so that runtime behavior is not capped by heuristic sidecars.
  - As a LLM 工程师, I want hard-fail error semantics with DecisionTrace, so that failures are explicit and diagnosable.
  - As a 回归测试人员, I want 100% runtime event/snapshot coverage, so that viewer outputs fully reflect runtime state.
- Critical User Flows:
  1. Flow-VIEWER-RUNTIME-LLM-FULL-001（真实 LLM 决策闭环）:
     `AgentChat/PromptControl -> auth 校验 -> AgentRunner 调用 LLM -> decision -> runtime action -> runtime event -> viewer event + DecisionTrace`
  2. Flow-VIEWER-RUNTIME-LLM-FULL-002（快照全量覆盖）:
     `runtime tick -> snapshot build -> WorldState 全字段映射 -> viewer snapshot -> UI/调试面板可观测`
  3. Flow-VIEWER-RUNTIME-LLM-FULL-003（硬失败语义）:
     `LLM timeout/网络失败 -> ActionRejected + DecisionTrace.error -> 无启发式 fallback`
  4. Flow-VIEWER-RUNTIME-LLM-FULL-004（协议扩展）:
     `runtime 新事件 -> 扩展 WorldEventKind/WorldSnapshot -> viewer 收到新事件/字段 -> 协议版本校验通过`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 真实 LLM 决策链 | `LlmAgentBehavior`、`AgentRunner`、`DecisionTrace` | llm 模式下所有决策只走 AgentRunner | `idle -> deciding -> decided/failed` | 决策按 tick 顺序执行 | 仅本地受控链路可写 |
| runtime 事件 100% 映射 | `DomainEvent` -> `WorldEventKind` | 所有事件必须映射或扩展协议 | `event_emitted -> mapped -> delivered` | 按 runtime 顺序保持单调 | 无远程写入口新增 |
| runtime 快照 100% 覆盖 | `WorldState` -> `WorldSnapshot` | 每 tick 输出全字段快照 | `snapshot_vN -> snapshot_vN+1` | 字段 1:1 映射，无默认丢弃 | 只读展示 |
| DecisionTrace 输出 | `prompt_digest/latency/actions/error` | 每次决策后推送 trace | `trace_pending -> trace_sent` | 以决策序号排序 | 仅 viewer 订阅 |
| 硬失败语义 | `ActionRejected::LlmFailed` | LLM 失败不触发任何 fallback | `deciding -> failed` | 超时按配置上限 | 保持 auth/nonce 规则 |
| 协议扩展 | `protocol_version`、新事件/字段 | 不兼容版本拒绝连接 | `hello -> version_check -> accepted/rejected` | 版本单调递增 | 本地启动可配置 |
- Acceptance Criteria:
  - AC-1: runtime live llm 模式完全移除启发式 sidecar；任何决策均来自 `AgentRunner<LlmAgentBehavior<_>>`。
  - AC-2: `DomainEvent` 枚举覆盖测试通过，映射覆盖率 100%。
  - AC-3: `WorldState` 快照覆盖测试通过，字段覆盖率 100%。
  - AC-4: LLM 调用失败/超时时返回 `ActionRejected::LlmFailed` + DecisionTrace.error，不产生任何动作。
  - AC-5: DecisionTrace 在每次决策后可被 viewer 订阅并包含 `prompt_digest/actions/latency/error`。
  - AC-6: viewer 协议支持新增事件/字段；协议版本不兼容时返回结构化拒绝。
  - AC-7: required 回归命令通过并可追溯到 `PRD-WORLD_SIMULATOR-019`。
- Non-Goals:
  - 不在本 PRD 中重写 viewer 前端 UI。
  - 不在本 PRD 中重构 runtime 规则系统或经济模型。
  - 不在本 PRD 中调整 LLM prompt 内容或策略优化（仅接管决策链路）。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - LLM 调用：复用 `simulator/llm_agent.rs` 现有 OpenAI 客户端与配置读取。
  - 观测面：shadow `WorldKernel` 与 `Observation` 工具链。
- Evaluation Strategy:
  - 决策链路：DecisionTrace 覆盖率 100%，失败率与错误码可回归。
  - 映射覆盖：DomainEvent/WorldState 覆盖测试必须为 100%。
  - 硬失败：LLM 错误与超时均走结构化拒绝路径，无 fallback 行为。

## 4. Technical Specifications
- Architecture Overview:
  - runtime::World 负责权威状态与动作执行；runtime live 维护 shadow `WorldKernel` 用于 LLM 观测与决策。
  - LLM 决策由 `AgentRunner<LlmAgentBehavior<_>>` 产生 `SimulatorAction`，再经 `simulator_action_to_runtime` 进入 runtime。
  - runtime 事件/快照被完整映射为 viewer 协议（必要时扩展 `WorldEventKind/WorldSnapshot`）。
  - DecisionTrace 与错误语义同步输出，确保可诊断。
- Integration Points:
  - `crates/oasis7/src/viewer/runtime_live.rs`
  - `crates/oasis7/src/viewer/runtime_live/control_plane.rs`
  - `crates/oasis7/src/simulator/llm_agent.rs`
  - `crates/oasis7/src/simulator/runner.rs`
  - `crates/oasis7/src/runtime/state.rs`
  - `crates/oasis7/src/runtime/world/domain.rs`
  - `crates/oasis7/src/viewer/protocol.rs`
- Edge Cases & Error Handling:
  - LLM API 失败/超时：返回 `ActionRejected::LlmFailed`，并输出 DecisionTrace.error。
  - 映射遗漏：单测失败阻止编译通过，不允许运行时静默丢弃。
  - 事件/快照字段新增：必须同步扩展协议并更新覆盖测试。
  - 版本不兼容：返回 `protocol_incompatible`，拒绝连接。
  - runtime action 不可执行：保留原 `ActionRejected::RuleDenied` 语义。
- Non-Functional Requirements:
  - NFR-1: 典型场景（<= 256 agents, <= 512 locations）快照构建 p95 <= 250ms（本地开发机）。
  - NFR-2: DecisionTrace 生成开销 p95 <= 5ms（不含 LLM 网络请求）。
  - NFR-3: LLM 失败错误在超时阈值后 1s 内返回结构化错误。
  - NFR-4: 新增/改造 Rust 文件单文件行数 <= 1200。
- Security & Privacy:
  - prompt/chat 鉴权、nonce anti-replay 与 agent-player 绑定校验不得退化。
  - DecisionTrace 不输出私钥或完整签名，仅保留 digest。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1: 文档建模与任务拆解。
  - M2: 真实 LLM driver + shadow kernel + 硬失败语义。
  - M3: 事件/快照全量映射、协议扩展与回归收口。
- Technical Risks:
  - 风险-1: runtime 与 shadow kernel 状态不同步导致决策偏差。
  - 风险-2: 协议扩展导致老 viewer 兼容性失败，需要版本门控。
  - 风险-3: LLM 调用不稳定导致硬失败频率上升，需确保错误语义可诊断。

## 6. Validation & Decision Record
- Test Plan & Traceability:
  - PRD-WORLD_SIMULATOR-019 -> TASK-WORLD_SIMULATOR-042/043/044/045 -> `test_tier_required`。
- Decision Log:
  - DEC-WS-015: 选择“真实 LLM + shadow kernel + 100% 映射 + 硬失败”，否决“保留启发式 fallback”。依据：硬失败是可诊断性与行为一致性的最小前提。
  - DEC-WS-016: 允许扩展 viewer 协议以承载 runtime 全量事件/快照，否决“维持旧协议并静默丢弃字段”。依据：100% 覆盖要求必须通过协议扩展实现。
