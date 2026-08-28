# LLM Agent Decision Provider 标准层 + Local Provider 外部适配可行性（2026-03-12）

- 对应设计文档: `doc/world-simulator/llm/decision-provider-contract.design.md`
- 专题入口与权威边界: `doc/world-simulator/llm/README.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前 world-simulator 的 Agent 能力已经具备 `Observation -> AgentBehavior::decide -> AgentDecision -> runtime 校验/执行 -> DecisionTrace` 的完整闭环，但“世界内 Agent 契约”与“具体 LLM / agent 框架实现”仍高度耦合。若未来要评估 `Local Provider` 这类外部 agent framework 参与游戏模拟，缺少一个稳定的中间标准层，会导致接入成本高、风险边界不清、验证口径分散。
- Proposed Solution: 在 world-simulator 的 Agent 层与具体 provider 之间维护 `Decision Provider` 标准层，冻结 `ObservationEnvelope / DecisionRequest / DecisionResponse / FeedbackEnvelope / TraceEnvelope` 这组内部契约；`Local Provider` 通过 adapter 作为可插拔外部 provider 接入，仅负责“决策建议与工具编排”，不替代 runtime 权威规则执行。当前已有 provider typed bridge 与显式 runtime seam；本轮设计升级把下一阶段重点放在 runtime 自动生成 subject-bound catalog/context、受信 semantic schema、typed args host encoding，以及未知制度 E2E 证明，不把这些目标误报为当前 production wiring。
- Success Criteria:
  - SC-1: 明确 `runtime authoritative / provider advisory` 边界，任何外部 provider 都不得直接修改世界状态或绕过 kernel 校验。
  - SC-2: 形成一套 provider-agnostic 的决策标准层契约，能够同时兼容现有本地 `LlmAgentBehavior` 与未来 `Local ProviderAdapter`。
  - SC-3: 定义 `Local Provider` 可行性评估矩阵，覆盖动作映射、工具编排、记忆策略、错误语义、时延/成本与可观测性。
  - SC-4: 形成阶段化落地路线：`MockProvider -> Local Provider PoC -> 低频 NPC 试点 -> 扩面评估`。
  - SC-5: 外部 provider 的非法输出、超时、格式错或 schema 漂移均有统一失败策略，并可映射到 `Wait` 或 `ActionRejected`。
  - SC-6: provider 输出可回写为 `AgentDecisionTrace`，保持 viewer / QA / replay 诊断链路连续。

### 1.1 能力状态与证明边界

本专题同时记录当前实现事实和目标合同，不能因为 DTO、校验器或单元测试存在，
就把完整的 Agent/runtime 生产闭环写成已交付。状态词的含义固定如下：

| 状态 | 本专题口径 |
| --- | --- |
| `current` | 当前代码已经存在、可由源码/测试直接观察到的行为；不暗示已经自动接入所有 live Agent。 |
| `partial` | 有可复用的类型、校验器、fixture 或显式 bridge seam，但缺少生产 wiring、语义 schema 或端到端证明。 |
| `target` | 本文冻结的后续设计合同；实现前仍需 runtime/WASM/QA 联审，不能作为现行安全保证。 |
| `proven` | 只有在同一 fixture 证明发现、typed args 编码、runtime 重验、提交反馈和失败无副作用后，才可标记。 |

当前与部分实现证据：

- `DecisionProvider`、`DecisionRequest/Response`、Mock fixture、内置/loopback adapter
  和 `AgentDecisionTrace` 桥已经存在；provider 仍是 advisory，runtime 才是权威执行者。
- active module manifest 可生成确定性、排序后的 `module_command_catalog`；v2
  `CapabilityCatalogSnapshot`、`CapabilityInvocationContext`、`AgentCommandResponse`
  的身份/快照/nonce 校验及显式 `RuntimeTrustedModuleExecutor` seam 已有代码与测试。
- `ProviderBackedAgentBehavior` 当前通过显式 `with_capability_context` 接收一份由调用方
  提供的 catalog/context；没有证据表明生产 live loop 会为每个 Agent turn 自动生成
  subject-bound snapshot、schema context 和 nonce 后注入所有 provider。现有 catalog
  entry 主要是 module/version/namespace/command/schema version/hash/payload bound，
  不包含可供 Agent 理解的参数/返回语义；`ModuleCommandEnvelope.payload` 是 opaque bytes。
- 因此，“typed provider bridge 已落地”只能描述校验和显式路由 seam；不能宣称未知制度
  已能被 Agent 自动发现、理解、调用，也不能宣称生产 auto-wiring 或完整 schema validation。

目标与证明要求：

- `runtime` 每个决策 turn 根据 active manifest、主体的未撤销授权、policy、world/branch
  和 audience 自动生成 subject-bound `CapabilityCatalogSnapshot` 与一次性
  `CapabilityInvocationContext`，再把不可变副本注入 `DecisionRequest`。provider 只能
  消费该副本，不能选择 subject、grant、audience、snapshot 或 nonce。
- catalog entry 必须能解析到 content-addressed semantic schema descriptor。descriptor
  至少包括命令说明、typed input 的字段/类型/必填性/范围或枚举、返回/错误结构、可见
  副作用和 bounded cost hint；descriptor 的 canonical bytes hash 必须与 entry 的
  `schema_hash` 一致。schema retrieval 失败或 hash 不一致时不得让 provider 猜测。
- Agent/provider 返回的是绑定 entry 的 typed args intent；可信 host 按已取回且验过的
  descriptor 将 args 编码成 deterministic/canonical CBOR，再形成 envelope。provider
  不得把自带的 raw payload、schema 或自然语言名称升级为权威参数。
- runtime 在 sandbox、计量、journal 或状态变更前重新校验 envelope canonical encoding、
  payload 对 descriptor 的完整解码、schema/version/hash、active module、subject/grant、
  live policy、cost quote、nonce/replay 和 payload bound。只有 committed result 才回写
  feedback/memory。
- “未知制度”验收必须使用一个不在 Kernel closed `Action` 枚举中的、仅由 active governed
  module 声明的 command，证明无需为该制度添加 native action 或让 Agent 获得 runtime
  写权限即可完成 discovery -> typed intent -> host encoding -> runtime commit -> feedback。
  在该 E2E 与负例矩阵通过前，能力仍是 `partial`。

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
     `provider timeout / malformed output / unknown tool -> adapter classify -> traceable Wait or catalog rejection -> DecisionTrace.error`。失败不得偷偷提交启发式替代动作。
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
| 受治理 capability discovery | `CapabilitySnapshot/CapabilityRef` | provider 看到安全 core/kernel 工具与当前 active module 的受限 projection | `snapshot_issued -> proposed -> binding_checked -> validated/rejected` | 每个 capability 绑定 namespace、version、schema ref、identity hash、caps 与 runtime cost quote | discovery 不授予写权；runtime 仍是唯一裁定者 |
| subject-bound discovery | `CapabilityCatalogSnapshot + CapabilityInvocationContext` | runtime 按 Agent subject/presenter/audience/grant/policy 自动生成本轮 catalog/context 并注入 request | `live_turn -> snapshot_issued -> provider_consumed -> expired` | snapshot identity/hash、world/branch/finality、revocation epoch、nonce 必须绑定；不允许调用方静态复用 | provider 只能回显绑定身份；catalog 暴露不授予权限 |
| semantic schema retrieval | `schema_ref + schema_hash + SemanticCommandSchema` | host 从受信 module manifest/schema store 取回 descriptor，校验 canonical bytes hash 后向 provider 暴露 | `schema_requested -> verified -> supplied/failed` | descriptor 版本、typed args、返回/错误结构、范围/枚举与 cost hint 可复现 | retrieval/hash/版本失败必须 fail closed，不接受 provider 自报 schema |
| typed args 编码 | `TypedCommandIntent -> canonical ModuleCommandEnvelope` | Agent 返回绑定 command 的 typed args；trusted host 校验并以 descriptor 编码 payload | `typed_intent -> host_encoded -> runtime_revalidated` | 编码必须 deterministic；字段缺失、类型/范围/枚举错误不得产生 payload | provider 不得注入 raw payload 或伪造 caller/grant |
| Provider 评估 | `valid_action_rate/timeout_rate/p95_latency/trace_completeness` | 使用固定 fixture 对比 provider 表现 | `bench_pending -> bench_done` | 按场景/agent 类型分层统计 | QA 可复核 |
| runtime-live 决策桥 | `AgentRunner/AgentBehavior/DecisionProvider/AgentDecisionTrace` | provider 只提出候选；runtime 校验并执行 | `observed -> proposed/wait -> validated -> executed/rejected` | trace、action、result 与事件保持因果关联 | provider 失败或未知动作不得触发替代启发式动作 |
- Acceptance Criteria:
  - AC-1: 建立 `Decision Provider` 标准层专题文档，明确数据契约、边界、风险与验证口径。
  - AC-2: 明确 `Local Provider` 的角色定位为“外部 provider / adapter”，而非 runtime / kernel 替代物。
  - AC-3: 文档中定义至少一条可执行 PoC 路线，要求先通过 `MockProvider` 与 fixture 验证，再进入 `Local Provider` 试点。
  - AC-4: 文档中冻结最小动作映射策略：只允许先在低频、低破坏性动作集上试点（例如 `wait`、`move`、`chat`、有限查询）。
  - AC-5: 文档中定义统一失败策略与 trace 回写规范，保持与 `AgentDecisionTrace`、`ActionRejected`、viewer 调试面一致。
  - AC-6: 文档中定义 required/full 验证矩阵，并可追溯到本专题项目任务。
  - AC-7: capability discovery 必须以安全 core/kernel surface 为稳定基线，并仅从当前 active module manifest/ABI projection 生成动态 capability；每个 projection 都绑定 namespace、version、schema ref、manifest/artifact/interface hash、caps 与 runtime cost quote。
  - AC-8: capability 未知、已撤销、未激活、权限不匹配、schema/identity hash 漂移或 cost quote 过期时，必须 fail closed，留下稳定 trace/error，并收敛为 `Wait` 或 `ActionRejected`；不得按名称猜测、静默改绑或执行 fallback。
  - AC-9: LLM/provider 输出只能是绑定上述 capability 的结构化 intent；intent 必须进入现有 runtime authoritative pipeline，由 runtime 重新校验并决定是否产生权威状态变化。
  - AC-10: 现有 closed `Action` adapter 保持可读写兼容并与动态 capability 共同进入同一执行管线；迁移按低风险 module 试点渐进进行，不以 capability discovery 取代现有 vertical slice。
  - AC-11: live Agent turn 的 catalog/context 由 trusted runtime 自动生成并 subject-bound；context digest 必须认证 world/branch/finality、policy/revocation epoch、snapshot digest 与 nonce，fixture 覆盖每一维漂移后 fail closed；不得把手工 `with_capability_context` 注入误报为 production auto-wiring。
  - AC-12: 每个动态 command 都能从 `schema_ref` 取得 semantic descriptor，并证明 descriptor canonical bytes 的 `schema_hash`、module declaration、version 与 catalog entry 一致；只做 envelope canonical CBOR 检查不足以满足本条。
  - AC-13: provider 只能返回绑定 entry 的 typed args intent；trusted host 负责类型/范围/枚举/必填校验和 deterministic canonical CBOR 编码，runtime 再做完整 schema、权限、预算、live state 与 replay 校验。
  - AC-14: unknown-institution E2E 复用 `institution-migration-v1` 与产品 SC-32，使用未进入 Kernel closed `Action` 的 active governed module command，证明 discovery -> schema -> typed args -> host encoding -> runtime commit -> committed feedback/memory；并验证 preview/stale/deny/no-effect 不扣费、accepted effect/debit/receipt exactly-once、retry/reconnect/restore/replay 不重复扣费或发放 replay credit。
  - AC-15: E2E 负例至少覆盖 unknown/inactive module、无 catalog/schema/hash 漂移、revoked grant、stale snapshot/policy、subject/presenter/audience mismatch、malformed typed args、oversize payload、expired cost quote 和 nonce replay；均产生稳定 trace/error 并收敛为 `Wait` 或 `ActionRejected`，不得名称 fallback 或静默重绑。
- Non-Goals:
  - 不在本轮直接把 `Local Provider` 接入主线模拟代码。
  - 不在本轮重写现有 `LlmAgentBehavior`、memory 系统或 runtime kernel。
  - 不在本轮把 `Moltbook` 等社交层能力引入 world-simulator。
  - 不在本轮把外部 provider 用于高频战斗/经济核心 actor。
  - capability discovery 不承诺任意 module 上传、source compile、安装或激活；这些仍由受治理的 runtime/WASM 生命周期决定。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - provider 必须支持结构化输入/输出，而不是仅依赖自由文本解析。
  - provider 必须能够暴露或适配“受限工具/动作白名单”，避免任意外部副作用。
  - provider 最好支持会话级 trace、tool trace、latency 指标与 message transcript 导出，便于映射为 `AgentDecisionTrace`。
  - tool 输入由稳定的安全 core/kernel tools 与 active module manifest/ABI projection 派生的动态 tools 组成；provider 不得自行添加、修改或声称激活工具，且 discovery 不授予安装、部署或上传权限。
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
  - `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`（Viewer live 观测与无伪进展边界）
  - `doc/world-simulator/prd.md`（runtime 事件、快照与恢复真值）
  - `doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`（operator HTTP-JSON interface；不是 `DecisionProvider` 或世界内 autonomous Agent）
- Standard Contracts:
  - `ObservationEnvelope`: `agent_id`、`world_time`、局部可见世界状态、近期事件摘要、记忆摘要、预算与动作白名单。
  - `DecisionRequest`: `observation + action_catalog + provider_config_ref + timeout_budget`。
  - `DecisionResponse`: `decision(wait/act) + action_ref + args + provider_error? + diagnostics + trace_payload`；`act` 仍只是候选 intent。
  - `FeedbackEnvelope`: `action_id + success/failure + reject_reason + emitted_events + world_delta_summary`。
- `CapabilitySnapshot`: 本轮 catalog 的 identity/digest；每个动态 capability 绑定 namespace、version、schema ref、manifest/artifact/interface hash、caps 与 runtime `cost_quote`。
- `TraceEnvelope`: provider transcript、tool trace、latency、token/cost、schema repair 记录。
- `SemanticCommandSchema`（target）：content-addressed descriptor，至少包含 `schema_ref/schema_hash/schema_version`、command summary、typed input fields（type/required/bounds/enums）、result/error shape、declared side effects 和 bounded cost hint；它是 Agent 理解输入的语义来源，不授予调用权限。
- `TypedCommandIntent`（target）：`catalog_snapshot_id + selected_entry + typed_args + response_nonce + trace_id`；其中 `selected_entry` 必须精确匹配 module/version/instance-target/namespace/command/schema，`typed_args` 不包含 caller、grant、authority proof 或 host-only provenance。module-instance audience 的 target 必须是稳定 `(world_id,module_id,instance_id)`；world/institution scoped command 使用显式非实例 variant，缺失值不得回退为全局 module。
- `CapabilityInvocationContext`：由 trusted host 为单次 turn 生成的 `grant_id/subject/presenter/audience/catalog_snapshot_id/module_id/module_version/instance_target/response_nonce`；provider 只能回显，不能自行构造为授权。
- context 的 canonical binding tuple 还必须包含 `world_id/branch_id/finality_epoch/finality_block_hash/revocation_epoch/policy_epoch/catalog_snapshot_digest`，并与 `response_nonce` 一起进入不可变 context digest。若兼容 DTO 不展开这些字段，`catalog_snapshot_id` 必须内容寻址且认证全部维度；runtime 提交前仍逐项与 live state 比较。
- 内置 Responses provider 的具体基线也由本专题承载，但不把某个 SDK、历史 JSON 多段执行策略或 Viewer surface 当作唯一 authority；provider-agnostic contract 仍是外部接入的唯一边界。
- 多场景评测必须固定并记录 scenario/fixture、agent profile、provider 与 adapter 版本、协议版本、timeout、tick budget 和 `--jobs` 等执行参数；同一评测 epoch 保留每场景 `report.json`、`run.log`、`summary.txt` 与聚合工件，避免总量掩盖单场景差异。
- `--jobs` 只描述执行并行度，不是行为等价或性能比较的充分条件。外部 provider 的非确定性运行必须在相同输入下重复采样；任一场景的 timeout、invalid output、stuck-loop、trace-completeness 缺口或未解释的错误不能因聚合均值/总量而被掩盖。
- 工业调试注入属于显式 debug capability，不属于普通 `ActionCatalog`、默认体验或 provider parity 样本。provider 只能选择当前 catalog 中已声明且经 runtime 校验的候选动作；资源、精炼 quote、经济与 replay 语义不由 provider contract 重定义。
- 已完成的历史多场景/工业长跑样本只保留为实现与风险演进证据；其旧阈值、prompt、计数或成功结论不得提升为当前 provider 成本、稳定性、parity 或发布准入证明。实际 parity/rollout 门槛以 `provider-agent-experience-parity.prd.md` 为准。

### 受治理 capability discovery 与兼容迁移

- catalog 保留安全 core/kernel tools 与现有 closed `Action` adapter；动态 tools 只由当前 active module manifest/ABI projection 派生。projection 是受限 provider 视图，不复制 WASM wire ABI，也不授予安装、部署、上传或激活权限。
- 每个 `CapabilityRef` 必须绑定 `namespace`、`version`、`schema_ref`、manifest/artifact/interface hash、声明的 `caps` 与 runtime 生成的 `cost_quote`，并受本轮 `CapabilitySnapshot` identity/digest 约束；provider 不得按名称或自然语言相似度重绑定。
- provider/LLM 只返回绑定 capability 的结构化 intent；runtime 重新做权限、schema、预算和实际成本校验，并将 intent 送入既有 authoritative execution/replay/receipt pipeline。工具成功不是 world fact，实际结果只来自 committed runtime feedback。
- 未知、revoked、未激活、版本/schema/hash/caps 漂移、snapshot 过期或 cost quote 缺失/过期/超预算时必须 fail closed，写稳定 trace/error 并收敛为 `Wait` 或 `ActionRejected`；不得 fallback、猜测重绑或生成替代动作。catalog 变化时旧 intent 失效，需重新 discovery。
- closed Action 与动态 module capability 共享同一 guard、执行管线和 trace；先用 mock projection/fixture 验证，再在低频低破坏性 module 上渐进迁移，保留旧 action 的兼容读写与 replay 路径，直到证据闭合。

### 受信 schema、typed args 与未知制度闭环（目标）

该闭环把“Agent 能看到 command”与“command 能安全执行”分开。推荐的单 turn 顺序为：

```text
runtime live state
  -> active manifest + subject grants/policy intersection
  -> subject-bound catalog + invocation context/nonce
  -> schema_ref retrieval + canonical schema_hash verification
  -> provider receives observation/context/catalog/schema
  -> typed command intent
  -> trusted host validates typed args and canonical-CBOR encodes envelope
  -> runtime revalidates identity/schema/authz/cost/nonce/live state
  -> staged module execution and committed result
  -> feedback + policy-governed memory write
```

`schema_ref` 是受信 descriptor 的内容寻址引用，而不是 provider URL 或自然语言别名。
host 只从 active module declaration 绑定的 manifest/schema store 取回 descriptor；取回
内容的 canonical bytes hash 不等于 entry 的 `schema_hash`、版本不支持、descriptor
缺字段，或 retrieval 超时，都应在 provider 调用前返回稳定的
`capability_schema_unavailable`/`capability_schema_mismatch`，不让 Agent 猜参数。
当前 `ModuleCommandCatalogEntry` 只提供机器 identity、版本/hash 和 payload bound，
`ModuleCommandEnvelope::decode_canonical` 只证明 envelope 的 canonical CBOR；它不证明
opaque `payload` 已按制度 schema 解码。这是必须保留的 partial 状态。

provider 的 target 响应是 `TypedCommandIntent`。host 根据已验证 descriptor 检查必填
字段、类型、范围、枚举和大小，按唯一 deterministic canonical-CBOR 规则编码 payload，
并填充 envelope 的 namespace/name/schema version/hash。provider 不能提交自带 raw bytes
来绕过该步骤，也不能把 `schema_hash`、grant、caller、subject 或 cost quote 作为普通
typed arg 覆盖。为兼容当前 `AgentCommandResponse.envelope`，raw envelope lane 可暂时
保留为 transitional/experimental；只有 host 重新 decode 并按 semantic descriptor 验证
后才允许进入 runtime，现有 envelope canonical 检查不应被描述为 semantic validation。

未知制度 proof fixture 应选择一个不在 Kernel closed `Action` enum 中、但已通过治理并处于
active 状态的 module command（例如 `economic_contract.open`）。在同一固定 world head
中，runtime 以 Agent subject 的已验证 grant 生成 catalog/context，取得 schema，provider
只返回 typed args，host 编码，runtime 完成 live recheck 和 staged commit；结果必须以
权威 `ActionResult`/module receipt 进入反馈，之后才允许写回 Agent memory。该 proof
不得依赖把 command 加进 native Action、静态全局 grant、provider 自报 schema 或手工
复制 context。它证明的是“未知制度可由受治理 module 接入”，不是该制度的玩法规则或
产品承诺。
该 fixture 复用 runtime `institution-migration-v1` 和产品 SC-32：preview、stale、deny 与 no-effect 不扣费，accepted effect/debit/receipt 仅结算一次，retry/reconnect/restore/replay 不重复扣费或发放 replay credit。provider feedback 只读取权威 receipt，不自行推断成本或结算。

未知制度 proof 的阻断条件和预期收敛：

| 条件 | 拒绝点 | 预期结果 |
| --- | --- | --- |
| module 未知、未激活或不在 snapshot | discovery/runtime live recheck | stable error + `Wait`/`ActionRejected`，无 sandbox/journal/state/charge |
| schema 缺失、hash/version 漂移或 descriptor 不可解码 | schema retrieval / host typed-args validation | fail closed，不调用 module |
| grant revoked/expired、subject/presenter/audience 不匹配 | runtime authorization | fail closed，不按名称或旧 grant fallback |
| snapshot/policy/branch/cost quote 过期 | runtime live recheck | old intent invalid，要求重新 discovery |
| typed args 缺字段、类型/范围/枚举错误或超 bound | trusted host encoding | 不生成可执行 envelope |
| nonce replay 或同 nonce 不同 request hash | runtime replay guard | idempotent historical receipt 或 deterministic denial，不重复副作用 |
| world/branch/finality、policy/revocation epoch 或 snapshot digest 漂移 | runtime context/live-state recheck | old context 失效并重新 discovery，不允许跨 authority boundary 重放 |

- Local Provider Adapter Strategy:
  - 使用 adapter 把 `ObservationEnvelope` 转成 `Local Provider` 可消费的会话输入。
  - 通过稳定 core/kernel tools 加上当前 active module 的受治理 capability projection 暴露 world 可执行候选，而不是开放任意外部命令。
  - adapter 负责把 `Local Provider` 的结果收敛为 `DecisionResponse`，并将 provider trace 映射到本地 trace 结构。
  - 首期 PoC 仅允许低频动作集，禁止直接接管高频 tick actor 与强一致经济关键路径。
- Edge Cases & Error Handling:
  - provider 超时：返回 `Wait`，同时记录 `llm_error=provider_timeout`。
  - provider 输出未知动作：由 action catalog guard 拒绝或收敛为 `Wait`，写入稳定 `error_code`；两者都不得提交状态变更动作。
  - provider 工具调用次数超限：adapter 终止会话并输出结构化错误。
  - capability 绑定或 catalog 在 discovery 后改变：尚未提交的 intent 失效并重新请求；已 committed action 只按权威 receipt/replay 继续，不重新请求 provider。
  - memory 写回冲突：以本地 memory 为准，provider 仅提交 write intent。
  - 网络故障 / API 401 / 插件不可用：按 provider_error 分类，不影响 runtime 主循环稳定性。
  - runtime 新增的事件/状态不得被 bridge 静默丢弃：协议以版本化 schema 或 exhaustiveness gate 失败；snapshot/event 保持因果顺序，DecisionTrace 可关联到 proposal、runtime result 与反馈。
  - 重连或 replay 不得重复执行已接受的动作；Viewer snapshot 只是观测/传输面，恢复仍以权威 state + journal/event chain 为准。
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
- Proof-first priority (architecture P0):
  - P0-A: 先用一个未进入 Kernel closed `Action` 的 active governed module 完成未知制度 E2E；该用例必须从 runtime discovery 到 committed feedback/memory 闭环，并保留所有负例的无副作用证据。
  - P0-B: 将手工注入的 `with_capability_context` seam 收敛为 runtime-owned per-turn catalog/context 生成和注入；在此之前只把 v2 typed bridge 视为 partial。
  - P0-C: 定义并实现 semantic schema retrieval 与 typed args -> trusted host canonical encoding；与 runtime 的统一 transaction、instance-targeted command/state 按同一 Institution Migration Test 依赖顺序协同推进，不以本专题重排 runtime 的架构 P0。
  - P0-D: 以固定 schema/catalog/subject/branch/预算上下文重复评测 provider；将 schema failure、context contamination、drift、stuck-loop、latency/token/cost 作为独立指标，不以聚合成功率掩盖单场景失败。
- Technical Risks:
  - 风险-1: `Local Provider` 的会话/工具模型与 tick 驱动 world actor 语义不完全同构，可能导致抽象层过宽或过厚。
  - 风险-2: 外部 provider latency/cost 抬高后，模拟规模与回归频率会受限。
  - 风险-3: provider memory 与本地 `AgentMemory` 双写可能造成上下文漂移。
  - 风险-4: tool schema 演进过快会让 adapter 成为新的维护热点。
  - 风险-5: 若没有 fixture 与统一错误签名，多个 provider 的评估将无法横向比较。

## 6. Validation & Decision Record

## 共识提交、结果反馈与重放

- 稳定决策生命周期是 `observe -> propose/wait -> catalog/schema guard -> submit when consensus-backed -> committed replay -> runtime execute/reject -> feedback`；这些词描述阶段，不新增持久 decision ledger 或协议枚举。
- consensus-backed live execution 中，proposal acceptance 或 submit acceptance 都不更新世界记忆；只有 committed action 被 replay 后的权威 `ActionResult` 才进入 `on_action_result`、recent summary 与后续决策上下文。
- provider failure、malformed output 或 unknown action reference 确定收敛为带 trace/error 的无状态变化 `Wait`，不得生成 heuristic substitute action。
- replay 从持久事件重建后果，不重新请求 provider，也不把 `replay_id` 当幂等授权。provider memory write 仍只是 policy-governed intent，不是世界事实。
- **当前 effect/receipt debt：** `AgentDecisionTrace` 的 module-call intent/receipt 与 journal event 是诊断事实，但尚未单独证明 intent 与权威 `ActionResult` 的完整因果映射、replay 时绝不重调 provider、或恢复时绝不重复外部副作用。T4/T5 必须补齐这组 runtime-owned proof；在此之前不得把 trace 的存在宣传为完整 receipt/replay closure。
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-036 | TASK-WORLD_SIMULATOR-112 | `test_tier_required` | `./scripts/doc-governance-check.sh` | 模块文档入口、专题索引、owner/边界定义 |
| PRD-WORLD_SIMULATOR-036 | T1/T2/T3/T4/T5 | `test_tier_required` / `test_tier_full` | fixture contract test + mock provider + future adapter PoC tests | agent provider abstraction、外部 provider 接入边界、trace 与 memory 桥接 |
| PRD-WORLD_SIMULATOR-036 | runtime-live bridge baseline | `test_tier_required` | event/snapshot schema exhaustiveness + provider timeout/invalid-output trace + reconnect/replay idempotence + state/journal recovery proof | runtime/Agent/Viewer 桥接；不得以 Viewer snapshot 作为恢复真值 |
| PRD-WORLD_SIMULATOR-036 | multi-scenario evaluation evidence | `test_tier_required` / `test_tier_full` | fixed scenario/fixture/profile epoch; retain per-scenario plus aggregate artifacts; repeated samples for nondeterministic providers | comparability, stuck-loop/error visibility, and parity evidence; historical long-run samples alone are insufficient |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-DP-001 | 以 `Decision Provider` 标准层隔离 world-simulator 与外部 agent framework | 直接让 `Local Provider` 替代现有 Agent 层 | 当前 runtime / trace / memory 契约已存在，直接替换风险过高且难以验证边界。 |
| DEC-DP-002 | 将 `Local Provider` 定位为 adapter/provider，而非 kernel/runtime 替代物 | 让 `Local Provider` 直接控制世界状态或规则执行 | runtime 权威与 replay/QA 契约必须保留在本地内核。 |
| DEC-DP-003 | 首期仅开放低频、低破坏性动作集做 PoC | 首轮就接入高频战斗/经济关键 agent | 先验证抽象层与 error policy，再决定是否扩面，能显著降低产品与工程风险。 |

## 内置 Responses Provider 基线与诊断 receipt 边界

- builtin provider 使用 OpenAI-compatible Responses transport；`base_url` 可规范化 API base、`/v1/`、旧 chat-completions 或 responses 后缀。配置解析优先级为 `[llm]` 直接值、选中 profile/provider、root、环境变量与 builtin default；agent override 以规范化 agent 名称选择。坏 TOML 必须显式失败，不能因单键可回退而静默吞掉整份配置错误。
- 未配置 `system_prompt` 时 builtin fallback 为“硅基个体存在的意义是保障硅基文明存续和发展；”。这只是可配置的 provider 指令默认值，不是世界规则、玩家承诺或对目标达成的保证。
- profile 是 `DecisionRequest.provider_config_ref` 的受控配置，而非世界记忆或兼容别名。未知 profile 必须结构化失败；不得静默改用通用 profile。单次 `timeout_ms` 是明确上限，有限 retry 只适用于可解释 provider error；超时、格式错误、预算耗尽、未知工具或非法动作都留下分类 trace/error，并收敛为无状态变化的 `Wait` 或 `ActionRejected`，不得生成启发式替代动作。
- builtin prompt 保持 tool-only：每 turn 只能查询或提交一个最终 `agent_submit_decision`，不以自由文本授权动作。`PromptBudget` 按优先级裁剪 observation、conversation/module history 与 memory digest；module calls、decision turns 与 repair rounds 均有上限。多个可解析片段只选择一个稳定终态并记录 collapse trace，绝不顺序执行同一 completion 的多个动作。
- 感知/上下文/短长期目标进入结构化 request；候选决策经 catalog/schema 与 runtime 校验后执行；只有 committed replayed `ActionResult` 回写 feedback、recent summary 和本地权威 memory。player、agent、tool、system chat role 分离；原始 provider JSON 不得伪装为玩家可读消息。外部 memory 只可提出 write intent，长期记忆影响玩家当前行动时的可读、可纠正合同仍由游戏专业 authority 证明，内部 history/trace 不能替代该证明。
- `agent_debug_grant_resource` 仅是显式 debug capability；普通 catalog、parity、成本与稳定性样本均不得使用它。token、延迟、prompt/completion 计数和有限 retry count 是诊断与评测输入，不是计费、发布或默认启用证据。
- `LlmEffectIntentTrace` / `LlmEffectReceiptTrace` 记录 `llm.prompt.module_call`、intent id、params、capability ref、origin、status、payload 与预留 cost；`max_module_calls` 控制体积，旧 trace 以默认字段兼容。这些是 simulator diagnostic facts：在 runtime-owned T4/T5 证明 intent-to-ActionResult causality、replay never re-calls provider、以及恢复不重复外部副作用前，不得宣称完整 effect/receipt 或 replay closure。
- simulator diagnostic event 名称固定为 `LlmEffectQueued { agent_id, intent }` 与 `LlmReceiptAppended { agent_id, receipt }`。写入顺序是：`LlmAgentBehavior` 处理 module call 时生成 intent/receipt，`AgentRunner::tick` 从 `AgentDecisionTrace` 读取，再逐条写入 `WorldKernel.journal`。该顺序仅描述诊断记录，不把 intent/receipt 提升为权威 effect、ActionResult 因果或 replay 幂等证明。
- provider 可表达 compile/deploy/install、offer/bid/purchase/delist/destroy/cancel 等模块意图，但 parser/schema 通过不等于动作已接受；所有意图仍经 runtime 权威校验、共识提交和结果回执。production 禁止 runtime source compile，只有显式 dev/test 可用受限 source package；actor、hash、版本、价格、订单标识与必填 payload 必须严格校验。`module.lifecycle.status` 只派生自已知 artifact/instance/ownership/listing/order/installed 状态，不是第二 registry，也不宣称激活、所有权或成交。
