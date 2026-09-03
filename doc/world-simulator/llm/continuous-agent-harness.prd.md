# Continuous Agent Harness：连续认知与 provider-neutral 决策合同（2026-08-31）

- 对应设计文档：`doc/world-simulator/llm/continuous-agent-harness.design.md`
- 上游 provider 边界：`doc/world-simulator/llm/decision-provider-contract.prd.md`
- 专题入口：`doc/world-simulator/llm/README.md`

## 1. Executive Summary

world-simulator 已有 `Observation -> AgentBehavior::decide -> candidate decision -> runtime validation/execution -> trace` 的局部闭环，但它还不是一个可持续、可恢复、可隔离的 Agent cognition harness。当前 builtin 与 ProviderBacked 两条路径在 continuation、memory 和 prompt/context 处理上存在行为分叉；DecisionRequest 没有足够的 turn/request identity；local bridge 的反馈与幂等边界也不足以支持多个 Agent 并行。

本专题冻结一个 provider-neutral 的 Continuous Agent Harness 合同。Harness 负责一次 Agent session 内的感知组装、记忆检索、目标/continuation 投影、provider 调用、候选决策规范化、反馈路由和 memory write intent policy；Runtime 仍是世界事实、动作语义、授权、前置条件、提交、receipt、replay 与最终副作用的唯一权威。Harness 的任何成功都不等于世界状态已经改变。

本文是 durable authority，不是任务板。实现状态、任务分工和验证历史仍属于 GitHub task truth；本文的 `current/partial/target/proven` 只描述能力证明边界，不描述任务进度。

## 2. 能力状态矩阵

状态含义固定为：`current` 是可从当前源码直接观察到的行为；`partial` 是存在 DTO、fixture 或显式 seam 但缺少完整 wiring/证明；`target` 是本文冻结的合同；`proven` 只有在本文验收矩阵的成功与负例均通过后才可使用。

| 能力 | current | partial | target | proven 条件 | 当前证据/限制 |
| --- | --- | --- | --- | --- | --- |
| cognition loop | `AgentRunner::tick` 同步调用 `AgentBehavior::decide` | runtime-live 有 sidecar/mailbox seam | 可独立推进世界的异步 turn 协议 | 等待 provider 时世界与其他 Agent 仍按 runtime 合同推进，且 turn 结果可关联 | `crates/oasis7/src/simulator/runner.rs`、`crates/oasis7/src/viewer/runtime_live/llm_sidecar.rs` |
| provider boundary | `DecisionProvider`、Mock/loopback 和 typed DTO 存在 | Builtin/ProviderBacked 使用不同 runner 分支 | 两者均实现同一 Harness lifecycle 和错误合同 | 同一 fixture 下 request、failure、feedback、memory 语义等价 | `decision-provider-contract.*` 已冻结 advisory/provider 与 runtime authority 边界 |
| identity/idempotency | `DecisionRequest` 有配置/profile 等字段但无完整 turn/request identity | `replay_id` 可作为 metadata 传递 | Session/Turn/DecisionRequest identity 与 canonical digest 绑定 | retry 保持同 key；新 turn、观察或 authority 输入必得新 key；collision fail closed | local bridge 已从 timeout-independent legacy DTO 计算 invocation key，并在 `H_v1` 派生前移除 `timeout_budget_ms`；host ProviderBacked adapter 已传递完整 outer V1 context/response lineage，Mock harness 有对应 proof。真实 HTTP/loopback outer V1 wire mapping 与 paired production artifact 尚未证明，仍保持 `partial`/no-production-proof |
| concurrency | 单一同步 runner 的调用序列隐含串行 | sidecar 有 pending/mailbox 状态 | 每个 Agent 至多一个 in-flight turn；不同 Agent 可并行 | 并发/重入 fixture 无双 provider call、双 action 或反馈串线 | 单 Agent 的隐含串行不等于跨 adapter 的协议保证 |
| feedback | `FeedbackEnvelope` 与 bridge feedback endpoint 存在 | trace/action/receipt 可桥接 | 反馈按 `agent/session/turn/request/action/receipt` 关联、分区、排序 | A/B Agent 交错反馈永不进入对方 context；late/unknown feedback 不产生 cognition effect | local `ProviderState` 的 `recent_feedback` 已按 `(agent_subject, agent_session_id)` 分区并校验 `feedback_seq`、gap 与 collision；real HTTP/paired-artifact 与端到端 production proof 仍缺失 |
| memory | builtin 有本地 AgentMemory；协议有 `memory_write_intents` | provider response 能携带 intent，但没有统一生产 consumer | retrieval 是带 digest 的 snapshot；write intent 经 Harness policy 且只在 authoritative outcome 后提交 | 未提交/拒绝/超时不会写入；同 intent retry exactly-once；越权 scope 被拒绝 | `memory_write_intents` 目前主要是协议字段 |
| goals | builtin config 使用 short/long goal 字符串，provider mission 有 summary | goal 可进入 request/context | P0 使用兼容性的 `GoalSnapshot`；后续再演进 GoalGraph | goal revision 纳入 digest，旧计划不会跨 revision 执行 | GoalGraph/belief memory 不在 P0 |
| continuation | builtin `ActiveExecuteUntil` 可在内存中跳过重复 LLM | continuation 与 Builtin 行为绑定 | Harness-owned bounded continuation，Builtin/ProviderBacked 共用 | world/goal/policy/observation digest 变化、reject 或 expiry 会停止 continuation | continuation 当前不是 runtime primitive，也未跨 provider 统一 |
| failure policy | timeout/invalid output 已有 trace/Wait/ActionRejected 方向 | adapter-specific 错误分类不完全统一 | 稳定 error code、无 heuristic fallback、无失败副作用 | 每类错误均有 deterministic terminal state 和负例副作用证明 | provider 错误不能成为 world fact |
| durable turn/replay | `AgentDecisionTrace` 是 diagnostics/observability payload | `replay_id` 可随 request 传递 | cognition journal/replay 与 runtime paired contract 对齐 | replay 不重新调用 provider、不重复 action/memory/effect | journal schema/实现明确留给 paired runtime docs |
| cognition economy | latency/token/cost 在 trace 中可见 | 可作评测输入 | cognition reserve/settle 单独设计，Harness 只传递预算与诊断 | 需独立 economy fixture 与 runtime receipt 证明 | P0 不引入 `CognitionLease` 或资源扣账 |

当前没有任何一行的目标能力可标记为 `proven`。存在的实现、DTO 或测试只能支持 `current`/`partial`，不能把生产闭环、回放闭合或多 Agent 隔离提前写成已交付。

## 3. 权威边界

### 3.1 Harness owns cognition

Harness 是 Agent 的认知边界，拥有：

- session、turn、request identity，以及 canonical input digest；
- observation/context 组装、裁剪和 memory retrieval snapshot；
- goal projection、有限 continuation 和 provider request 生命周期；
- provider adapter 选择、候选决策规范化、repair/timeout budget 和诊断 trace；
- feedback 的 identity matching、顺序、隔离和可见性；
- `MemoryWriteIntent` 的大小、scope、来源、dedupe 与提交门槛；
- provider-neutral 的错误分类、降级到 `Wait`/`ActionRejected` 的行为。

Harness 可以提出候选动作、目标变更或记忆写入意图，但不能直接写 world state、receipt、authority、grant、资源余额或玩家承诺。

### 3.2 Runtime owns truth

Runtime 是世界事实与副作用的唯一权威，拥有：

- live world/branch/finality head、实体身份、授权、policy 与 capability validity；
- action 的语义、precondition、资源扣除、提交/拒绝、receipt 和事件顺序；
- stale/MVCC、调度、持久化 journal、replay 与恢复的最终合同；
- 哪些反馈是 committed world fact，以及哪些事实可以进入权威 Agent memory。

Runtime 不能把 provider transcript、模型自报目标完成或未提交的 intent 当成世界事实。Harness 不能以 timeout、旧 context、自然语言名称或 provider 自带 schema 绕过 Runtime。

### 3.3 连续认知流

```text
runtime observation/head
  -> Harness opens Session/Turn
  -> retrieval + GoalSnapshot + bounded continuation
  -> canonical DecisionRequest
  -> exactly one provider adapter request at a time
  -> candidate decision / Wait / structured failure
  -> Runtime validates and commits or rejects
  -> correlated FeedbackEnvelope
  -> Harness updates continuation/diagnostics and proposes memory writes
```

上图中的 Runtime 调度、动作 MVCC、journal 和 receipt 持久化不是本文的实现合同；它们必须由配对的 runtime 文档另行冻结。本文只要求 Harness 在这些边界上使用稳定 identity，并在未取得权威结果时保持无副作用。

## 4. Session、Turn 与 DecisionRequest identity

### 4.1 身份定义

| identity | 生成者 | 生命周期 | 用途 |
| --- | --- | --- | --- |
| `agent_session_id` | trusted Harness host（Harness 唯一分配者） | 一个 Agent 的连续 cognition session；重启/迁移按恢复合同决定是否延续 | feedback/memory 分区和 session-level diagnostics |
| `agent_turn_id` | trusted Harness host（Harness 唯一分配者） | 一个 observation-to-outcome cycle | 将一次上下文、候选决策和权威结果绑定起来 |
| `decision_request_id` | trusted Harness host（Harness 唯一分配者） | 一个逻辑 provider request；transport retry 必须复用 | provider dedupe、retry 和 replay identity |
| `transport_attempt` | adapter/transport | 每次网络尝试 | 诊断和 retry 计数；不改变逻辑 request identity |
| `feedback_id` / `feedback_seq` | Runtime host | 一条权威反馈及其 session 内顺序 | 防重、排序、late/unknown feedback 分类 |

Provider 只能回显 host 颁发的 identity；不能自行选择 Agent subject、session、turn、request、catalog、nonce、grant 或 receipt identity。`decision_request_id` 不是每次 retry 新建的随机 key；retry 必须保留同一逻辑 request。

Harness 独占 cognition ID 的分配（session/turn/request）；Runtime 只验证、持久化并在其自身域分配 journal、action/envelope、receipt、continuation/wake 等 runtime IDs。任何一方都不得从 process ID、wall-clock 或 provider session key 猜测另一方的 identity。

### 4.2 与既有 Decision Provider 契约的兼容映射

Continuous Harness 不复制或替换上游 `decision-provider-contract.prd.md` 已冻结的
`DecisionRequest/DecisionResponse/FeedbackEnvelope`。它增加一个版本化的 outer cognition
context（实现上可称 `ContinuousAgentRequestContextV1`），把 session/turn/request identity、
runtime binding、digest、budget 和 Harness policy 作为 additive extension 绑定到既有
provider request；既有 observation、profile/config、fixture/replay、capability 和 tagged
decision variants 仍按上游契约解释。response 同样以 outer identity envelope 包住既有
candidate/wait/query/module-response payload，不制造第二套动作语义。

兼容期的 adapter 必须能把 outer context 映射为既有 provider DTO，并在 response 返回时
验证并回显 outer identity；缺少扩展字段的旧 fixture 只能走明确标记的 compatibility lane，
不能进入 production async claim。target path 以 outer context 的 canonical encoding 计算
共享 `request_digest`，但 Runtime/provider 仍消费同一上游 decision semantics。

outer wire 必须带 `context_discriminator = "oasis7.continuous-agent-context"` 与
`context_version = 1`。兼容 adapter 的字段映射固定如下：

| 既有 inner provider DTO | outer Harness context / response | 规则 |
| --- | --- | --- |
| `DecisionRequest.observation` | `base_decision_request.observation` + `observation_digest` | observation 语义不变；digest 由 trusted host 计算 |
| `DecisionRequest.action_catalog` | `base_decision_request.action_catalog` + capability/catalog digest | provider 只能消费 host 注入的 snapshot |
| `DecisionRequest.provider_config_ref` | `base_decision_request.provider_config_ref` + profile revision | 不得因缺失 profile 静默换 profile |
| `DecisionRequest.timeout_budget` | `base_decision_request.timeout_budget` + normalized `budget_contract` | timeout 不单独产生 identity |
| `DecisionResponse.decision/action_ref/args` | `base_decision_response` | tagged `Wait/WaitTicks/Act` candidate semantics，不复制动作字段 |
| `ProviderDecision.Query { query_ref, query }` | `base_decision_response.decision` | frozen-snapshot read-only query；产生 `AgentQueryResult`，无 action/envelope/world effect |
| `ProviderDecision.ModuleCommandResponse { response }` | `base_decision_response.decision` | host 校验 catalog/context/nonce 后交 Runtime；不转成 core `Action` |
| `DecisionResponse.module_command` | `base_decision_response` | 作为 typed module candidate；必须经 host schema encoding 与 Runtime validation，不是已执行 action |
| `DecisionResponse.provider_error/diagnostics/trace_payload` | `base_decision_response` + bounded outer trace | 诊断不得升级为 world fact |
| `DecisionResponse.memory_write_intents` | `base_decision_response`，再经本 PRD 的 memory policy | provider intent 不是直接 write |

Target outer wrapper 必须保留 `ProviderDecision` 的 tagged discriminator，覆盖 `Query` 与
`ModuleCommandResponse`，不得只实现 `act/wait` 子集。`DecisionResponse.module_command?` 仍是
additive compatibility field；旧 adapter 若把 `ModuleCommand` 投影到该字段，必须显式记录
projection，不能覆盖或替代 `ModuleCommandResponse`。Query 结果与 typed module response 都
沿用相同 session/turn/request/digest correlation 和 Runtime receipt boundary。

`context_version` 只有实现了对应 canonical schema 才可协商；未知 version 或缺少
discriminator 必须返回 `unsupported_context_version`。未知 outer authority 字段必须
fail closed 为 `unknown_context_field`；未来 additive 字段须进入新版本并纳入 digest。旧
inner DTO 的已知非-authority diagnostics 可由兼容 adapter 丢弃，但不得悄悄纳入 identity；
缺少 outer context 的 legacy fixture 只能记录 `legacy_no_cognition_proof`，不能进入
production async claim。

Legacy compatibility lane 必须由 adapter route 或 fixture/profile 显式选择；缺省和 production
async lane 均使用 target outer contract。该 lane 只允许把旧 DTO 映射到 outer fields；不得将
缺少 cognition proof 的 response 标为 target/proven。当前 HTTP wire 不定义
`compatibility_lane` 字段。任何 legacy heuristic fallback 仅限低风险 fixture/profile，
不得进入 production async claim、target parity 或自动记忆/continuation 语义。

### 4.3 Canonical request digest

Harness 为每个逻辑 request 计算跨 Harness/Runtime 共享的 `request_digest`。provider 的
调用幂等 key 是该 digest 的 domain-separated 派生值，不是另一套 request identity：

```text
provider_invocation_key = H_v1(
  domain="oasis7.cognition.provider-invocation.v1",
  request_digest=request_digest,
)
```

`canonical(request_identity)` 至少包含：

- protocol/digest version、`agent_session_id`、`agent_turn_id`、`decision_request_id`、semantic `retry_seq` 和 agent subject；
- `world_id`、branch/finality identity、`base_tick`/base world head；
- observation canonical bytes 的 digest；
- capability catalog/invocation context digest（若本 turn 没有 capability，也要有明确 empty value）；
- memory retrieval snapshot digest、GoalSnapshot digest 和 continuation digest；
- provider config/profile revision、adapter/protocol version；
- 本 request 的 bounded decision budget（timeout/token/repair budget 的规范化值，若它是 provider contract 的一部分）。

canonical encoding 必须固定字段顺序、长度前缀、UTF-8 规范化、map key 排序和缺省值；list 默认保留语义顺序，只有 schema 明确声明 order-insensitive/set-like 时才按 canonical element bytes 排序，未声明语义不得猜测排序。不允许用 wall-clock、endpoint、secret、transport attempt 或原始 provider output 参与 identity。配置、world head、观察、能力 context、memory 或 goal 任一 authority input 变化，必须创建新 request，不能静默重绑。

`request_digest` 的输入固定为 `ContinuousAgentRequestContextV1` 去掉输出字段
`request_digest` 和 transport-only 字段 `transport_attempt` 后的 canonical bytes，并使用
domain `oasis7.cognition.request.v1`；response outer 中回显的 request digest 永不进入自身
输入。实现不得把空字符串、JSON null 或 provider 返回的 digest 当作缺失字段的替代。

同一 `decision_request_id + request_digest` 的重试必须得到同一逻辑结果或同一结构化失败；同一 request id 搭配不同 digest 是 `request_identity_collision`，必须 fail closed，不能覆盖旧结果。timeout 不能再单独构成幂等 identity。Runtime action/module envelope 的 `envelope_digest/envelope_idempotency_key` 是另一条 runtime-owned identity，必须在 paired runtime docs 定义，不能复用 `provider_invocation_key`。

Harness 与 Runtime 的 canonical wire mapping 使用 paired runtime PRD 的
`Cross-contract identity mapping`：`agent_subject == agent_id`，session/turn/request 使用
`agent_session_id/agent_turn_id/decision_request_id`，`runtime_binding` 完整映射 world、
branch/finality、base tick/hash、reorg epoch 与 runtime manifest hash。adapter 不得自行创造
别名或从缺失字段猜测 identity。

### 4.4 Session/Turn state mapping

Harness 状态与 Runtime 状态/处置的最小映射固定为：

| Harness state | Runtime state/disposition | 语义 |
| --- | --- | --- |
| `Preparing` | `queued` / `running` | Harness 分配 cognition IDs；Runtime 验证并持久化 envelope 后才运行。 |
| `InFlight` | `waiting_provider` | 一个逻辑 request 一个活动 provider attempt，受 Runtime lease 约束。 |
| `Candidate` | `response_recorded` / `ready_to_submit` | Harness 规范化 candidate；Runtime 仍验证 authority、precondition、envelope。 |
| `AwaitingReceipt` | `accepted` / `rejected_*` / `failed_*` / `recovery_pending` | 只等待 Runtime authoritative disposition。 |
| `Wait` | `pending` | Agent 选择无副作用等待；保留因果 identity 等待 wake/recovery；不并发开启同 Agent 下一逻辑 turn。 |
| `Completed` | `committed` | 仅匹配 receipt 可结束；随后按 policy 处理 memory/continuation。 |
| `Stale` | `rejected_stale` / `recovery_pending` | head/lease/precondition/digest 失效，丢弃 candidate 并清除不兼容 continuation。 |
| `Failed` | `failed_*` | 终止逻辑 request，保留 bounded failure trace，禁止 heuristic fallback。 |
| `Cancelled` | `cancelled -> rejected` with `reject_reason=cancelled` | 终止 turn、释放 single-flight、清除 continuation；迟到 response 按 `late_response_after_cancel` 规则进入 `stale/expired` diagnostic。 |
| `Pending` | `scheduler_backpressure` / `recovery_pending` | 保留同一 identity 等待 capacity/recovery；不调用 provider、不开新逻辑 turn。 |

`retry_seq` 是 semantic retry 序号；新逻辑 turn/request 递增并建立因果引用。相同逻辑
request 的 transport retry 只递增 `transport_attempt`，不改变 `request_digest` 或
`provider_invocation_key`。Runtime lease expiry 必须以 `pending` 或 `failed_*` 收口 active
turn、释放 single-flight，并使迟到响应进入 `stale/expired` diagnostic 或既有 receipt 路径；
不能重新打开 turn。Session 只允许 `active -> draining -> closed`，崩溃恢复在 journal 证明
同一逻辑 request 时保留 identity，冲突则进入 `recovery_pending`；`closed` session 不接收
cognition 或 feedback。

Runtime 的 `scheduler_backpressure` 必须释放本地 attempt lease、保留 bounded pending record，
并在 capacity/recovery wake 后只恢复原 identity；`cancelled`、rejected 或 failed 均清除不可复用
continuation。Runtime canonical feedback status 仍只有 `committed/rejected/failed/pending`，
因此 `cancelled` 以 rejected reason 映射给 Harness，而不是泄漏为第五个 status。

状态 precedence 固定为：`Wait` 只表示 Agent 的无副作用决定；Runtime
`scheduler_backpressure` 或 `recovery_pending` 一旦产生，Harness 进入 `Pending` 并覆盖本地
Wait 标签。`Pending` 不是第二种 feedback status，也不得与 `Wait` 合并为重复的 state row。

取消后的响应映射固定为四层：Runtime 内部 disposition/code 为
`late_response_after_cancel`；若 Runtime 生成 FeedbackEnvelope，字段为
`status=rejected`、`reject_reason=late_response_after_cancel`，且无 receipt/effect；Harness
将其映射为已关闭 turn 的 `stale`（若 lease/retention 已过期则为 `expired`）bounded
diagnostic，保留原 `feedback_id/feedback_seq` 和 reason。该诊断不是新的 AgentTurn status，
不得把 `cancelled` turn 重新打开，也不得进入 prompt、memory 或 continuation。

AgentTurn 的 target status enum 固定为 `preparing | in_flight | awaiting_receipt |
completed | rejected | cancelled | stale | failed | pending`；`Rejected` 和 `Cancelled`
都是 terminal Harness states，分别承接 Runtime `rejected_*` 与 `cancelled` disposition。
状态图固定为 `Preparing -> InFlight -> (Candidate -> AwaitingReceipt -> Completed |
Rejected | Cancelled | Stale | Failed | Pending; Wait -> Pending; Rejected; Cancelled)`。
两者都释放 per-agent single-flight、清除不可复用 continuation；lease expiry 或
`scheduler_backpressure` 必须以 `Pending` 或 `Failed` 收口 active turn。terminal 后的迟到
response 只能进入 `stale/expired` bounded diagnostic，不能重新打开 turn。

## 5. Per-agent single in-flight rule

1. 每个 `agent_subject` 至多有一个 active session；该 session 至多有一个 active turn；一个 turn 至多有一个 active provider attempt。session id 只用于分区，不能放宽 per-agent 全局互斥。retry 只能串行发生，不能并发复制请求。
2. provider 返回 candidate 后，在 Runtime receipt/deny 解决该 turn 前，Harness 不得为同一 Agent 开启下一 turn。Runtime 是否继续推进世界由 Runtime 调度合同决定，不由 provider 阻塞整个 world。
3. 新 observation 在 active turn 期间必须被标为 `deferred/coalesced`，不能偷偷替换 request context；若变化使 request stale，则关闭旧 turn 并要求新 turn 重新 discovery/context。
4. 不同 Agent 可以并行；隔离键是 Agent/session，而不是一个全局 provider queue 或全局 recent feedback buffer。
5. 违反 single in-flight 的调用收敛为 `agent_busy`/`protocol_violation`，不得产生第二个 action、第二份 memory commit 或第二条可见反馈。

## 6. Provider abstraction convergence

Builtin Responses/provider 和 ProviderBacked/Local adapter 是同一 Harness 的 provider implementation，而不是两套不同的 world-agent lifecycle。它们必须共用：

- 同一 `DecisionRequest`、`DecisionResponse`、`FeedbackEnvelope`、trace 和 error code；
- 同一 identity/digest、single in-flight、timeout/retry、memory policy 和 continuation boundary；
- 同一 Runtime candidate-action handoff；provider-specific transcript/tool protocol 只存在于 adapter 内部；
- 同一 MockProvider/golden fixtures，且 provider 差异通过 latency、token、valid-action、trace completeness 等评测指标表达。

现有 Builtin-only `ActiveExecuteUntil`、prompt-profile path 或 ProviderBacked 显式 context 构造均属于迁移输入，不能继续成为不同安全语义的旁路。统一后的 Harness 可以保留兼容 DTO，但 production path 必须通过本文 identity 和 feedback contract。

Provider trace bounds 固定为：每 turn 最多 64 条 transcript、64 条 tool trace；单条分别最多
2 KiB、1 KiB；input/output summary 各最多 1 KiB；upstream trace 最多 4 KiB；canonical
trace payload 总计最多 32 KiB。非-authority trace 超限返回 `trace_payload_too_large` 并只
丢弃诊断 artifact，不改变合法 candidate 的 Runtime disposition；authority-bearing overflow
是 `protocol_violation`。credential/token/authorization/private-key/path 字段统一替换为
`<redacted>`；无法安全识别的敏感值 fail closed，不做静默截断。required fixture 必须覆盖
超限、敏感字段和 redaction。

## 7. Feedback correlation 与 isolation

每条 `FeedbackEnvelope` 至少绑定：`agent_session_id`、`agent_turn_id`、`decision_request_id`、`request_digest`、candidate `action_id`（若有）、Runtime `receipt_id`（若已产生）、`feedback_id`、`feedback_seq`、canonical `status`、reject reason、emitted events/world delta summary 的脱敏投影。若已形成 envelope，还应携带 `envelope_digest` 以便恢复和 duplicate verification。

Runtime disposition 到 feedback 的映射由 paired runtime lifecycle 文档冻结（见
[`agent-cognition-lifecycle.prd.md`](../../world-runtime/runtime/agent-cognition-lifecycle.prd.md)
与 [`agent-cognition-lifecycle.design.md`](../../world-runtime/runtime/agent-cognition-lifecycle.design.md)）：
canonical status 仅为 `committed/rejected/failed/pending`：`committed` 必须带
committed receipt/projection；stale、expired、authority、precondition 与 idempotency conflict
统一使用 `rejected` 加稳定 `reject_reason`；provider/persistence 终态失败使用 `failed`；未决
恢复使用 `pending`；Runtime 明确确认成功处理但没有世界变化时也使用 `rejected` 加
`reject_reason=no_effect`；`no_effect` 不可作为独立枚举值，不能替代 duplicate 或覆盖原 disposition。
Harness 只能消费该 authoritative
disposition，不能从 provider HTTP status、到达顺序或 transcript 自行推断反馈。

Harness 只把同时匹配 Agent/session、active turn/request、expected action/receipt lineage 且 sequence 合法的反馈放入下一次 cognition context。A/B Agent 的反馈必须物理分区；不得由一个全局 `VecDeque<String>` 或等价全局 prompt buffer 提供跨 Agent 上下文。

`feedback_seq` 按 Runtime session sequence 单调消费：gap 只进入 bounded hold buffer，不进入
prompt；缺口超过 retention boundary 返回 `feedback_sequence_gap`，不按到达时间重排。相同
sequence+digest 是幂等 duplicate；相同 sequence+不同 digest 是 `feedback_identity_collision`，
必须 fail closed。

- late、unknown、duplicate 或 cross-agent feedback：写入 bounded diagnostic trace，不能进入 Agent memory/context，也不能改变 continuation；active turn 仍由 Runtime 的 pending/failed disposition 或 lease expiry 收口，不能永久停留 active；
- receipt 未提交的 provider transcript：不是 feedback，不得提升为 world fact；
- feedback 顺序由 Runtime receipt/sequence authority 决定，Harness 不按到达时间重排世界事实；
- 反馈内容必须脱敏、大小受限，并禁止携带 credential、grant proof 或未经授权的其他 Agent 私密观察。

旧 `FeedbackEnvelope` v1 的 field-level mapping 固定为：`action_id -> candidate_action_id`；
`success=true` 只有在 Runtime 提供 committed receipt 时才映射为 `status=committed`；
`success=false` 必须携带 Runtime disposition 才能映射为 `rejected`/`failed`/`pending`，
否则 fail closed 为 `legacy_feedback_ambiguous`；`reject_reason` 作为稳定 reason，
`emitted_events/world_delta_summary` 仅进入 bounded、redacted projection。旧 DTO 缺少 session/turn/request/digest/
receipt correlation 时只能进入 `legacy_v1` lane，不能关闭 target turn 或进入 target
memory/continuation。

Feedback projection 上限固定为 canonical envelope 16 KiB、`emitted_events` 32 项、事件摘要
512 UTF-8 bytes、world delta summary 1024 UTF-8 bytes。超限产生
`feedback_projection_too_large`，只保留已验证的 identity/status/reason，不静默截断或注入
context/memory；redaction 使用固定 `<redacted>` marker，无法安全识别的敏感 payload 直接
拒绝 projection。gap/乱序、超限和敏感字段必须有 required negative fixtures。

## 8. Memory retrieval 与 write-intent policy

`MemoryContextSnapshot` 是只读输入，必须带 digest 并参与 request identity。外部 provider memory 只能是缓存/检索辅助，不能替换本地 Agent memory 的权威副本。

无 memory entry 时使用 schema-defined empty snapshot（`revision=0`、空 entries、显式 scope
和 canonical digest）；不得用缺字段、JSON `null` 或空字符串替代。空 snapshot 不阻止
decision，但任何 write intent 仍须通过本节 policy 和 committed outcome gate。

`MemoryWriteIntent` 是候选写入，不是 write command。Harness 必须：

- 只接受当前 Agent/session/turn 来源，校验允许的 scope、summary/tag 格式、大小、数量和敏感字段；
- 为每个 intent 计算 digest，按 `(agent_session_id, agent_turn_id, intent_digest)` 去重；
- 仅在同一 turn 获得 Runtime authoritative committed outcome 后，按本地 memory policy 提交；timeout、malformed、stale、deny、uncommitted 或 identity collision 不得提交；
- 将 world fact 与模型推断/偏好标注不同 provenance；没有 Runtime receipt 的“目标已完成”“余额变化”等内容不得写成事实记忆；
- 跨 Agent、共享世界记忆、玩家可见记忆和长期记忆的语义/授权必须由对应专业 authority 另行批准，P0 不默认开放。

Memory write 的 exactly-once、持久化与 replay 由 paired runtime/journal 合同证明；本文不把已有 DTO 或 trace 视为已消费链路。

target memory candidate 的 `MemoryWriteIntentV1` wire schema 固定为：

```text
MemoryWriteIntentV1 {
  schema_version: 1,
  scope: turn_private | session_private | agent_private_long_term,
  summary?,                         // optional; explicit presence encoding
  tags: string[],                   // omitted tags encode as explicit empty list
}
```

`intent_digest` 是 Harness 按 policy 计算的 derived artifact，不是 provider 可自报的 authority。
现有 inner `MemoryWriteIntent { scope: String, summary: String, tags: Vec<String> }` 保持不变，
仅能经显式选定的 legacy route/adapter 映射；target lane 不按旧 DTO shape 猜测版本。当前
HTTP wire 不定义 `compatibility_lane` 字段。

P0 的 `MemoryWriteIntentPolicyV1` 冻结为：scope wire enum 为
`turn_private | session_private | agent_private_long_term`，但 P0 默认 allowlist 只有
`turn_private | session_private`；`agent_private_long_term` 默认返回 `memory_scope_denied`，
延后到 P2/独立 memory authority，不属于 P0 proven 行为。每 turn 最多 8 个 intent，
每个 summary 最多 512 UTF-8 bytes，每个 intent 最多 8 个 tag、每个 tag 最多 64 bytes，
canonical intent bytes 总计最多 4096 bytes。target policy 中 `summary` 是可选字段：省略时
使用显式 `present=false` 编码；若出现则先做 Unicode NFC、trim，必须非空且不得含 NUL/控制
字符。`tags` 可以是显式空列表或按 schema 省略（省略固定编码为 canonical empty list）；
每个 tag 先 NFC/trim 后必须非空且不得含 NUL/控制字符，随后按 canonical UTF-8 bytes 排序
并去重。`intent_digest` 的输入为
`intent_digest = H_v1("oasis7.cognition.memory-write-intent.v1", canonical(payload))`，其中
payload 固定包含 policy version、Agent/session/turn、`request_digest`、scope、canonical
summary presence/value、canonical tags 和 host-assigned provenance；排除 intent_digest 自身、
transport attempt 与 raw provider transcript。adapter 不得用空 summary、JSON null 或缺失字段
替代显式的 summary presence encoding。稳定
拒绝码至少包括 `memory_source_mismatch`、`memory_scope_denied`、
`memory_intent_count_exceeded`、`memory_summary_too_large`、`memory_tag_invalid`、
`memory_tag_count_exceeded`、`memory_payload_too_large`、`memory_sensitive_content`、
`memory_no_committed_outcome`、`memory_intent_duplicate` 和 `memory_digest_mismatch`。
provider 不得自报 `runtime_fact` provenance；只有 Runtime committed receipt 才能授予
world-fact provenance。上述 policy 是 Agent-private P0 gate，不等于 GoalGraph 或共享记忆。
兼容 lane 可把旧 `scope=short_term` 显式映射为 `turn_private` 并记录
`memory_scope_alias_used`；旧 DTO 的 summary 必须为非空字符串。target lane 不接受该 alias，
必须返回 `memory_scope_denied`；空 tags 仍允许，但空 tag element 必须返回 `memory_tag_invalid`。
只有显式 future authority/feature gate 才可开启 `agent_private_long_term`，仍不属于 P0 target。

## 9. Goal 与 continuation 边界

P0 将既有 short/long goal 字符串及 provider `goal_summary/blocked_reason` 归一为只读的 `GoalSnapshot`（含 revision、摘要、blocked 状态和 digest），以便 request identity 稳定。projection precedence 固定为 trusted host Agent config/runtime goal projection > legacy provider `goal_summary/blocked_reason`；provider 字段不能覆盖已有 host 值。各摘要先 NFC+trim，short/long 各最多 512 UTF-8 bytes，blocked_reason 最多 256 bytes；超限返回 `goal_snapshot_too_large`，不得静默截断。无值使用 revision=0、空摘要、无 blocked_reason 的显式 empty snapshot。Harness 可以提出 goal update intent，但 Runtime/world outcome 才能证明目标是否达成。

完整 GoalGraph、依赖关系、belief memory、长期偏好纠正和玩家可见目标语义明确 deferred beyond P0，不能以字符串升级为已完成 GoalGraph。

Continuation 的 policy、proposal、bounds、advance/invalidate 语义由 Harness 拥有；它仍然只是 bounded cognition hint，而非 Runtime transaction 或 world truth。Runtime 拥有 durable schedule、wake、crash recovery 和最终执行时机。Continuation 至少绑定：原始 turn/request、action/plan kind、remaining budget、baseline observation/goal/policy digest、允许的 precondition summary 和 expiry。只有 Runtime 接受并返回可继续的 outcome，Harness 才能减少 budget 并继续；stale head/observation/goal/policy、reject、timeout、expiry 或 identity mismatch 必须清除/暂停 continuation 并重新决策。Builtin 与 ProviderBacked 必须共享该语义。

Harness 提交唯一 wire schema `ContinuationProposalV1`；它由 Harness 分配
`continuation_proposal_id`，并使用 canonical `origin_request_digest` 绑定来源 request，禁止
`request_digest` wire alias。其余字段和 Runtime 派生规则以同名 design schema 为准：
`AgentContinuation` projection 必须携带 Runtime 的 `world_id/branch_id/finality_epoch/
finality_block_hash/finality_status/reorg_epoch/runtime_manifest_hash` binding，以及
`decision_request_id/origin_request_digest` lineage；不得
由 Harness 添加与 Runtime 不一致的 proposal alias。Runtime 验证 proposal 后分配自己的
`continuation_id`、`wake_id`、`wake_seq` 和 `next_wake_tick`，持久化为 `AgentContinuation`；
Runtime 的 schedule/recovery fields 不能被 provider 或 Harness 伪造。Runtime paired contract
负责最终 wake/disposition，Harness 只根据该 disposition advance 或 invalidate proposal。

`FinalityBindingV1` 与 Runtime 共享如下唯一 branch/finality 子结构，不新增 proposal wire
字段：`FinalityBindingV1 { schema_version, branch_id, finality_epoch: u64,
finality_block_hash?: Digest32, finality_status: pending | verified | reorged | suspended,
reorg_epoch: u64 }`，其 binding
digest 使用 `H_v1("oasis7.runtime.finality-binding.v1", canonical(FinalityBindingV1))`。
`world_id` 与 `runtime_manifest_hash` 是同一 continuation projection 的 world/schema binding；
reorg、finality 或 world drift 必须由 Runtime 以稳定 rejection 收口。

`proposal_digest` 与进入下一次 request 的 `continuation_digest` 使用共享 H_v1 registry：

- `proposal_digest = H_v1("oasis7.cognition.continuation-proposal.v1", canonical(proposal_without_proposal_digest))`；
- `continuation_digest = H_v1("oasis7.cognition.continuation-context.v1", canonical(context))`，
  其中 context 固定包含可选 continuation_proposal_id/proposal_digest、Runtime 分配的可选
  continuation_id/wake_id/wake_seq、`world_id`、`branch_id`、`finality_epoch`、
  `finality_block_hash?`、`finality_status`、`reorg_epoch`、`runtime_manifest_hash`、
  remaining_budget、valid_until_tick、`status: ContinuationStatusV1`、
  `continuation_status_digest` 和 terminal_disposition；
  该字段集合与 paired Runtime contract 一致。

`ContinuationStatusV1` 的枚举、允许迁移以及
`H_v1("oasis7.cognition.continuation-status.v1", ...)` 输入完全以 paired Runtime contract 为准；
Harness 只能消费 Runtime-owned status projection，不能生成或改写 `continuation_status_digest`。

没有 continuation 时必须编码同一 schema 的全 absent canonical object，不能使用空字符串、
缺字段或 adapter 自选 null。digest 自身永不进入自己的输入 bytes。

## 10. 错误与失败语义

| 条件 | Harness terminal result | 副作用保证 |
| --- | --- | --- |
| request identity/context/catalog/schema stale 或 mismatch | `Stale` / `NeedsRediscovery` | 不提交 action、memory 或 effect |
| provider timeout/transport failure | bounded retry（同 request id）；耗尽后 `Wait(provider_unavailable)` | 不产生 heuristic action；保留脱敏 trace |
| malformed/unknown/disallowed decision | `ActionRejected(invalid_decision)` 或 `Wait` | 不进入 Runtime commit，不消费 memory intent |
| budget/repair limit exceeded | `Wait(budget_exhausted)` | 不把 repair 文本当 action |
| duplicate same digest | 原结果/原失败的 idempotent replay | exactly-once；不重复 provider side effect |
| same request id with different digest | `ProtocolViolation(request_identity_collision)` | fail closed；不覆盖历史结果 |
| unmatched/late/cross-agent feedback | diagnostic-only discard | 不污染 context/memory/continuation |
| Runtime `late_response_after_cancel` | `stale` / `expired` diagnostic | 保留 feedback identity/reason；不重新打开 turn，不进入 context/memory/continuation |
| invalid memory write intent | `memory_write_rejected` trace | 只丢弃该 intent，不改变 world state |
| Runtime deny/reject | `ActionRejected(runtime_denied)` | 只有 Runtime 定义的拒绝 receipt 可见；不得 fallback |
| Runtime cancellation | `ActionRejected(reason=cancelled)` | terminal；释放 single-flight、清除 continuation；迟到 response 只走 stale/expired |
| scheduler backpressure | `Wait(scheduler_backpressure)` | 保留原 identity；不调用 provider、不产生 effect，capacity wake 后恢复原 request |
| Runtime confirms handled with no world effect | `ActionRejected(reason=no_effect)` | status=`rejected`；不写 memory、不推进 continuation、不生成 effect |

任何 provider failure 都不得偷偷替换为启发式动作。可用的安全降级是无副作用 `Wait` 或结构化 `ActionRejected`；重试必须受 bounded budget 限制。

## 11. P0-P2 rollout

### P0.1：Identity、隔离与共同合同

- 冻结 Session/Turn/DecisionRequest outer context、shared `request_digest`、`provider_invocation_key`、single in-flight 和稳定错误码；
- 让 local bridge/loopback 的 provider key 不再依赖 timeout，并按 Agent/session 隔离 feedback；
- 为 Builtin/ProviderBacked 建立共同 request/response/feedback fixtures，保留旧 DTO 仅作明确兼容输入；
- 建立 memory write intent 的验证、dedupe、提交门槛和 GoalSnapshot/continuation 的 bounded 语义。

### P0.2：Async actor + action MVCC coupled seam

- Harness 的 async AgentActor/single-flight 协议必须与普通 action 的 base-head/precondition/stale disposition 一起验证；
- Runtime scheduler、action MVCC 的具体数据结构和调度实现仍由 paired runtime docs 冻结；本阶段只消费其 binding、reject/stale disposition 和 receipt seam；
- P0.2 未通过前，不得声称 production async cognition。

### P0.3：Minimum journal/recovery seam

- 证明 active turn/request、provider invocation result、Runtime disposition 和 memory intent disposition 在 crash/retry/reconnect 后可恢复且不重复副作用；
- durable journal/recovery 的事件 schema、存储和 replay 实现细节留给 paired runtime docs；Harness 只要求 identity-bound read/write seam；
- P0.1、P0.2、P0.3 全部 required 通过，才可提出 production async claim。

### P1：Provider convergence 与 durable continuation

- 统一 Builtin/ProviderBacked 到同一 Harness lifecycle 和 host-side continuation policy/proposal；
- 通过 runtime-owned observation/head、durable wake/recovery 与 authoritative disposition 建立端到端 feedback/memory 回路；
- 扩展 deterministic loopback、多 Agent 交错、断线/retry/restore fixtures；
- P1 仍不授予 GoalGraph、belief memory 或 cognition economy 的生产承诺。

### P2：Advanced goals、memory、economy 与试点

- 在 P0/P1 seams 具备证明后，执行低频 NPC/未知 governed command full E2E 和固定 epoch 多样本评测；
- Cognition economy（reserve/settle/receipt）、GoalGraph、belief memory、共享/玩家可见记忆分别通过自己的 authority 与验收后再扩展；
- 未通过单场景负例或成本/稳定性门槛时保持 experimental，不默认启用高频战斗或经济核心 Agent。

## 12. Acceptance Criteria

- AC-HARNESS-01：文档明确 Harness cognition 与 Runtime truth 的 ownership；任何 provider 成功都不能直接写世界。
- AC-HARNESS-02：每个 logical request 具备 Harness-issued session/turn/request identity，并以 additive outer context 映射既有 provider DTO；transport retry 复用 request id。
- AC-HARNESS-03：shared `request_digest` 的 canonical 输入、字段顺序、map/list 规则和排除项固定（包括排除 `request_digest` 与 `transport_attempt`）；`provider_invocation_key = H(domain, request_digest)`，timeout 不再是 identity，并有完整 outer-context golden bytes/digest/key fixture。
- AC-HARNESS-04：同 request retry 得到同 key/同 logical outcome；新 turn、observation、world head、memory、goal 或 capability context 生成新 digest；collision fail closed。
- AC-HARNESS-05：每 Agent 至多一个 active session、一个 in-flight turn/attempt；不同 Agent 可并发；重入不会双调用、双 action 或双 memory commit；P0.2 的 async actor 与 action MVCC 必须共同通过。
- AC-HARNESS-06：Builtin 与 ProviderBacked 使用同一 provider-neutral lifecycle、error、feedback、memory 和 continuation contract。
- AC-HARNESS-07：feedback 绑定并隔离 Agent/session/turn/request/digest/action/receipt；交错 A/B 反馈不得 cross-talk；late/unknown/duplicate、超限、敏感和乱序仅 diagnostic；Runtime `late_response_after_cancel` 必须映射为 `stale/expired` diagnostic；cancelled/backpressure、Runtime pending/failed 或 lease expiry 均能收口 active turn且清理 continuation。
- AC-HARNESS-08：memory retrieval 带显式 empty digest；target `MemoryWriteIntentV1`（optional summary、empty-list tags encoding）与 `MemoryWriteIntentPolicyV1` 的 scope（P0 默认仅 turn/session-private，long-term disabled/deferred）、short_term legacy alias、8/512/8/64/4096 limits、canonicalization、stable reject codes、provenance/dedupe 受约束，只有 authoritative committed outcome 后才可提交；inner legacy DTO 仅 compatibility lane。
- AC-HARNESS-09：Harness 拥有 GoalSnapshot/continuation policy 与 proposal，Runtime 拥有 durable schedule/wake/recovery；stale/reject/expiry 会停止旧计划；GoalGraph/belief memory 明确 deferred。
- AC-HARNESS-10：timeout、malformed、stale、identity collision、provider unavailable、runtime deny/no_effect、cancelled、scheduler_backpressure 都有稳定失败码和无副作用保证；heuristic fallback 仅限显式 legacy lane且不计入 target proof。
- AC-HARNESS-11：P0.1 identity、P0.2 async actor+MVCC、P0.3 minimum journal/recovery 全部 required 后，才可提出 production async claim；P1 live convergence/durable continuation、P2 advanced systems 分层；不得用 DTO、trace 或 aggregate metrics 冒充 `proven`。
- AC-HARNESS-12：cognition economy 不在 P0 隐式实现；任何 reserve/settle 或计费承诺必须由独立 runtime/economy 合同和 receipt 证明。

## 13. Verification Matrix

| 层级 | 目标 fixture/命令类别 | 必须证明 | 不能证明 |
| --- | --- | --- | --- |
| `required` | canonical identity/JSON round-trip、完整 outer-context digest golden vectors（含不同 `transport_attempt`）、same-retry/different-turn、collision | identity 稳定、兼容序列化、幂等 key 不串 | provider 真实能力、Runtime commit/replay |
| `required` | per-agent concurrency harness、retry/timeout、bounded repair | single in-flight、无双调用、稳定 Wait/Rejected | world scheduler 非阻塞或 MVCC 正确性 |
| `required` | A/B interleaved feedback、late/unknown/duplicate feedback | correlation、分区、无 context contamination | Runtime receipt 持久化 |
| `required` | feedback/trace bounds、credential redaction、gap/overflow、cancelled/backpressure、legacy module/feedback mapping | bounded projection、稳定 overflow、终态 cleanup、无启发式 target 漂移 | 真实 provider 的行为质量 |
| `required` | memory intent size/scope/provenance/dedupe fixtures | 未提交不写入、非法 intent 拒绝 | durable journal exactly-once |
| `required` | MockProvider 对 Builtin/ProviderBacked common contract | abstraction convergence 的协议等价性 | 两个真实模型的行为 parity |
| `full` | loopback + Runtime authoritative receipt、stale head/deny/retry/restore | action/feedback/memory 因果和无重复副作用 | 更大规模玩家体验或默认启用 |
| `full` | paired scheduler/MVCC/journal/replay fixtures | 世界独立推进、旧 action 拒绝、replay 不重调 provider | Cognition economy/GoalGraph 产品正确性 |
| `rollout` | 固定低频 NPC scenario，多次 nondeterministic provider samples | valid-action、p95 latency、timeout、cost、trace、stuck-loop | 跨 scenario 的笼统平均替代单场景 gate |
| `proven` | 上述全矩阵成功与负例同一 fixture epoch 的完整 artifact | 才能把相应 target 标为 proven | 不能推导未覆盖的 module、provider 或玩家承诺 |

### 13.1 可执行验证入口

| 层级 | 命令 | 预期证据 |
| --- | --- | --- |
| docs current | `./scripts/doc-governance-check.sh` | PRD/design 配对、入口与文档治理通过 |
| P0.1 focused target | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_provider_local_bridge -- decision_identity` | golden digest、same retry/new request、collision 与 feedback partition fixtures 通过 |
| P0.1 provider target | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib provider_loopback_adapter` | outer context、legacy adapter、identity round-trip 与 failure mapping 通过 |
| P0.2 paired runtime target | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib agent_cognition_mvcc` | async actor handoff、stale/expired/precondition、capability/idempotency rejection 无 world effect |
| P0.3 paired runtime target | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib agent_cognition_recovery` | journal edge crash/retry/restore 后不重调已记录 provider、不重复 receipt/effect |
| required implementation gate | `./scripts/ci-tests.sh required` | required-tier Harness/provider/runtime contract 回归通过 |
| full proof gate | `./scripts/ci-tests.sh full` | multi-Agent、disconnect/retry/restore、provider parity 与长时稳定性通过 |

focused target 名称是对应实现切片必须建立的稳定 test filter；命令不存在或未执行时，
本表只定义验收入口，不构成 `proven`。输出至少归档 fixture bytes、digest/key、provider
invocation count、feedback partition 和 terminal disposition。

## 14. 明确 deferred 与 follow-up ownership

- Runtime scheduler、world-independent progress、普通 action MVCC/stale/precondition、durable cognition journal/replay，以及 disposition-to-feedback mapping：由 paired `runtime_engineer` 文档和验证 slice 冻结，本专题只消费其 identity/receipt seam。
- Cognition economy 的 reserve/settle、预算资源和扣账 receipt：P0 之外的独立 economy authority。
- GoalGraph、belief memory、长期偏好纠正、共享/玩家可见 memory：分别由产品/玩法/Agent authority 联审，不能在本合同中隐式扩大。
- Viewer transcript/diagnostic delivery、QA release judgment、repository Codex adapter/runtime dispatch：不属于本专题 ownership。
