# Continuous Agent Harness：连续认知与 provider-neutral 决策设计（2026-08-31）

- 对应需求文档：`doc/world-simulator/llm/continuous-agent-harness.prd.md`
- 上游 provider 契约：`doc/world-simulator/llm/decision-provider-contract.design.md`
- 专题入口：`doc/world-simulator/llm/README.md`

## 1. 设计定位与状态

本设计把 Agent 从“一次同步 decide 调用”提升为可关联、可隔离、可恢复的连续 cognition boundary。它不把 provider 变成 runtime，也不把已有 DTO、trace 或显式 constructor seam 误报成生产闭环。

| 状态 | 本设计口径 |
| --- | --- |
| `current` | 当前源码可观察到同步 `AgentBehavior::decide`、Builtin/ProviderBacked 分支、builtin-only in-memory continuation、诊断 trace 与 provider DTO。 |
| `partial` | 存在 `DecisionProvider`、loopback、capability context、`MemoryWriteIntent`、sidecar pending/mailbox 等可复用 seam，但 identity、反馈隔离、provider convergence 和生产 consumer 尚不完整。 |
| `target` | 本文的 Session/Turn/request identity、canonical digest、single in-flight、共同 adapter lifecycle、feedback/memory policy 和 bounded goal/continuation 合同。 |
| `proven` | 仅当 PRD verification matrix 的 required/full/rollout artifact 证明对应成功与负例，才可标记；本文不预先标记任何 target 为 proven。 |

## 2. 组件与 ownership

```text
                 ┌──────────────────────────────┐
                 │      Runtime authoritative    │
                 │ head / policy / action /     │
                 │ receipt / journal / replay   │
                 └──────────────┬───────────────┘
                                │ observation + receipt
                 ┌──────────────▼───────────────┐
                 │       Continuous Harness      │
                 │ session/turn, memory read,    │
                 │ goal/continuation, correlation│
                 │ single-flight, policy/trace   │
                 └──────────────┬───────────────┘
                                │ common DecisionProvider contract
                 ┌──────────────▼───────────────┐
                 │    Builtin / ProviderBacked   │
                 │      provider adapters        │
                 └──────────────────────────────┘
```

Harness owns cognition state and protocol safety. Runtime owns live truth and every authoritative side effect. Adapter owns transport, model selection, provider-specific transcript and bounded retry mechanics, but not identity authority or action execution.

The current `RuntimeDecisionRunner::{Builtin, ProviderBacked}` split is treated as a migration shape. A target implementation may keep these Rust types internally, but both arms must enter the same Harness lifecycle rather than silently defining separate memory, continuation or error semantics.

## 3. Canonical data model

### 3.1 Session and turn

```text
AgentSession {
  agent_session_id,
  agent_subject,
  protocol_version,
  profile_revision,
  opened_from_runtime_head,
  status: active | draining | closed,
}

AgentTurn {
  agent_session_id,
  agent_turn_id,
  retry_seq,                         // semantic retry sequence
  transport_attempt,                 // delivery attempt; excluded from request digest
  base_runtime_head,
  observation_digest,
  memory_snapshot_digest,
  goal_snapshot_digest,
  continuation_digest,
  status: preparing | in_flight | awaiting_receipt | completed | rejected | cancelled | stale | failed | pending,
}
```

`agent_session_id`、`agent_turn_id` 和 `agent_subject` 必须由 trusted Harness host 唯一生成；Runtime 只验证、持久化这些 cognition IDs，并分配其自身的 journal/action/envelope/receipt/continuation/wake IDs。provider 不能把 session 当作可移植的 bearer authority，也不能在跨 Agent 时复用 session。

### 3.2 DecisionRequest / response additive outer context

Continuous Harness 不复制上游 decision-provider contract 的 request/response。既有 provider
DTO 作为 inner decision payload 保持兼容；Harness 增加版本化的
`ContinuousAgentRequestContextV1` outer context，绑定 session/turn/request identity、
runtime binding、digests、budget 和 policy。兼容 adapter 把 outer context 映射到既有
`DecisionRequest` 字段并在 response 外层回显 identity；旧 fixture 若没有该 context，只能
留在明确的 compatibility lane，不能满足 production async claim。target provider response
仍返回既有 tagged candidate/wait/query/module-response semantics，不产生第二套 action contract。

```text
ContinuousAgentRequestContextV1 {
  base_decision_request: ExistingDecisionRequestV1, // upstream DTO, unchanged
  context_discriminator: "oasis7.continuous-agent-context",
  context_version: 1,
  protocol_version,
  agent_session_id,
  agent_turn_id,
  decision_request_id,
  retry_seq,                         // semantic retry; new request/turn
  transport_attempt,                 // transport retry only; excluded from digest
  agent_subject,
  runtime_binding: {
    world_id, branch_id, finality_epoch, finality_block_hash?, finality_status,
    base_tick, base_world_hash, reorg_epoch, runtime_manifest_hash
  },
  observation_digest,
  capability_catalog_digest,
  capability_invocation_context_digest,
  memory_snapshot_digest,
  goal_snapshot_digest,
  continuation_digest,
  adapter_protocol_version,
  budget_contract,
  request_digest,                     // output; excluded from request digest input
}

ContinuousAgentResponseContextV1 {
  base_decision_response: ExistingDecisionResponseV1, // upstream DTO, unchanged
  context_discriminator,
  context_version,
  agent_session_id,
  agent_turn_id,
  decision_request_id,
  retry_seq,
  transport_attempt,
  request_digest,
}
```

`outcome`、candidate、memory intents、continuation update 和 trace 仍属于
`base_decision_response` 的既有 semantics；outer context 只增加身份、binding 和 digest，
不复制动作字段。inner `DecisionRequest/DecisionResponse` 的 identity 必须通过 outer context
完整匹配。provider 回传不同 identity、缺字段或未知 version 时，Harness 产生稳定
`response_identity_mismatch`，不尝试从自然语言或旧 response 推断动作。

既有 inner DTO 的字段映射为：`observation`、`action_catalog`、`provider_config_ref`、
`timeout_budget` 保持在 `base_decision_request`；`decision/action_ref/args`、
`provider_error/diagnostics/trace_payload` 与 `memory_write_intents` 保持在
`base_decision_response`。outer 只添加 discriminator/version、cognition IDs、runtime
binding、digests、budget 与 policy。版本不支持或未知 authority 字段必须 fail closed；已知
非-authority diagnostics 可由兼容 adapter 丢弃且不进入 digest。legacy 无 outer context
只能标记 `legacy_no_cognition_proof`。

兼容 adapter 的 field-level mapping 固定为：

| ExistingDecisionRequest/Response field | V1 location | compatibility rule |
| --- | --- | --- |
| `observation`, `action_catalog` | `base_decision_request` + corresponding digests | semantics unchanged；snapshot/digest 由 trusted Harness host 提供。 |
| `provider_config_ref`, `timeout_budget` | `base_decision_request` + profile/budget fields | 缺 profile 不得静默换 profile；timeout 不单独构成 identity。 |
| `decision`, `action_ref`, `args` | `base_decision_response` | 保持 candidate/Wait/Rejected semantics，不复制动作字段。 |
| `module_command` | `base_decision_response` | 作为 typed module candidate；必须经 host schema encoding 与 Runtime validation，不是已执行 action。 |
| `provider_error`, `diagnostics`, `trace_payload` | `base_decision_response` + bounded outer trace | diagnostics 可按兼容策略丢弃，但不进入 identity digest。 |
| `memory_write_intents` | `base_decision_response` | 仍是 intent，必须经过 `MemoryWriteIntentPolicyV1` 和 Runtime outcome gate。 |

Target outer response 保留 `base_decision_response.decision` 的 tagged variant，不把 Query 或
module response 压扁为 `act`：

| Existing `ProviderDecision` variant | Target mapping | Harness/runtime semantic |
| --- | --- | --- |
| `Wait` / `WaitTicks { ticks }` | `base_decision_response.decision` | 无副作用等待；保留同一 outer identity。 |
| `Act { action_ref, action }` | `base_decision_response.decision` | candidate，仍须经 Runtime action validation。 |
| `Query { query_ref, query }` | `base_decision_response.decision` | 只读 query；使用当前 frozen world snapshot，产生 `AgentQueryResult`，不得生成 action/envelope、memory fact 或 world effect。 |
| `ModuleCommand { module_command }` | `base_decision_response.decision`；旧 adapter 可按明确 projection 放入 `base_decision_response.module_command?` | typed module candidate；没有 trusted executor 时收敛为 `Wait`/`module_command_unroutable`，不得伪装为执行成功。 |
| `ModuleCommandResponse { response }` | `base_decision_response.decision` | 完整 `AgentCommandResponse`；Harness/host 先校验 catalog、subject/audience、nonce/context，再交 Runtime；不转成 core `Action`。 |

`DecisionResponse.module_command?` 是 additive extension field，不能替代或覆盖 tagged
`ModuleCommandResponse`；outer wrapper 对两种模块 variant 都回显同一 cognition IDs/digest。
Query/result 与 typed module response 的结果仍属于该 turn 的 bounded response/feedback lineage，
不会绕过 `ContinuousAgentResponseContextV1` 或 Runtime receipt boundary。

`context_discriminator` 必须等于 `oasis7.continuous-agent-context`，`context_version=1` 仅在
对应 schema 已实现时协商；缺失 discriminator、未知 version 或未知 outer authority field
都 fail closed（分别为 `unsupported_context_version` 或 `unknown_context_field`）。未来
additive field 只能进入新版本并纳入 digest；旧 inner DTO 没有 outer context 时只记录
`legacy_no_cognition_proof`，不能进入 production async lane。

Legacy compatibility lane 必须由 adapter route 或 fixture/profile 显式选择；缺省和 production
async lane 均使用 target outer contract。该 lane 只允许把旧 DTO 映射到上述 outer fields；不得
把缺少 cognition proof 的 response 标成 target/proven。当前 HTTP wire 不定义
`compatibility_lane` 字段。任何 legacy heuristic fallback 仅限低风险 fixture/profile，
不得进入 production async claim、target parity 或自动记忆/continuation 语义。

### 3.3 FeedbackEnvelope

```text
FeedbackEnvelope {
  feedback_id,
  feedback_seq,
  agent_session_id,
  agent_turn_id,
  decision_request_id,
  candidate_action_id?,
  runtime_receipt_id?,
  status: committed | rejected | failed | pending,
  request_digest,
  envelope_digest?,
  reject_reason?,
  committed_event_summary?,
  world_delta_summary?,
  provenance: runtime_authoritative,
}
```

Runtime 负责声明 `committed/rejected/failed/pending`；stale、expired、authority、
precondition 与 idempotency conflict 都使用 `rejected` status 加稳定 `reject_reason`，而不是
发明第二套 status；没有世界效果使用 `rejected` 加 `reject_reason=no_effect`，而不是新的
status。其 disposition-to-feedback mapping 以 paired runtime lifecycle 文档为准
（见 [`agent-cognition-lifecycle.prd.md`](../../world-runtime/runtime/agent-cognition-lifecycle.prd.md)
与 [`agent-cognition-lifecycle.design.md`](../../world-runtime/runtime/agent-cognition-lifecycle.design.md)）。Harness 只负责匹配、脱敏、排序和进入下一次 cognition context。无 `runtime_authoritative` provenance 的 adapter transcript 不是反馈。

## 4. Canonical digest 与幂等算法

### 4.1 Digest domain

共享 `request_digest` 的 canonical domain 使用版本化、长度前缀的 deterministic encoding。它是 Harness 与 Runtime 共同消费的 cognition input digest；provider invocation key 只是它的 domain-separated 派生值。字段顺序为：

1. domain/version；
2. session/turn/request/subject identity；
3. world/branch/finality/base tick/head；
4. observation digest；
5. capability context/catalog digest；
6. memory retrieval snapshot digest；
7. GoalSnapshot 与 continuation digest；
8. provider config/profile revision 与 adapter protocol version；
9. normalized budget contract。

Object key 按 canonical byte order 排序；list 默认保留 schema 的语义顺序，只有 schema 明确声明
order-insensitive/set-like 时才按 canonical element bytes 排序；没有声明时不得猜测排序。字符串使用
normalized UTF-8；缺省值必须显式编码。wall-clock、transport endpoint、credentials、raw provider output、
日志顺序、`transport_attempt` 和本地 mailbox 地址都排除在 digest 外。

```text
request_digest_input = ContinuousAgentRequestContextV1 without request_digest and transport_attempt
canonical_request_bytes = canonical(request_digest_input)
request_digest = H_v1(
  domain="oasis7.cognition.request.v1",
  payload=canonical_request_bytes,
)
provider_invocation_key = H_v1(
  domain="oasis7.cognition.provider-invocation.v1",
  request_digest=request_digest,
)
```

`H_v1` 使用 paired runtime contract 冻结的 `blake3-256` registry entry：输入为
RFC 8949 deterministic CBOR 编码的 `[domain, payload]` 二元数组，输出为
`blake3:<64 lowercase hex>`。算法自检向量固定为
`BLAKE3-256(empty) = af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262`。
P0.1 必须再提交包含完整 `ContinuousAgentRequestContextV1` 的 immutable golden
fixture，并以不同 `transport_attempt` 值证明 canonical bytes、`request_digest` 和
`provider_invocation_key` 不变；在该 fixture 和 paired runtime 输出逐字节一致前，不得把
digest contract 标为 `proven`。

Runtime action/module envelope 另有 runtime-owned `envelope_digest/envelope_idempotency_key`，用于提交和副作用幂等；它不等于也不替代 `request_digest` 或 `provider_invocation_key`，具体定义留给 paired runtime docs。

Observation、memory、goal、capability schema 等大字段以已经验证的 content digest 进入 identity；digest 的来源 bytes 必须可从 fixture 或 trusted host 复现。若 digest 版本、字段或 authority input 不可解释，拒绝 request，而不是使用兼容猜测。

### 4.2 Retry 与 collision

- transport retry 保持 session/turn/request id 和所有 canonical inputs 不变，`transport_attempt` 单独递增；
- semantic retry 必须递增 `retry_seq`，创建新的 turn/request，并保留对旧 turn/request 的 causal reference；
- adapter cache/dedupe 以 `provider_invocation_key` 保存终态或稳定 failure；
- same key/same digest 返回原结果；same request id/different digest 生成 `request_identity_collision`，并使该 turn fail closed；
- 新 observation、runtime head、profile revision、goal/memory/capability digest 或 budget contract 变化必须开新 turn/request；
- 不允许再使用 `{session_key}-{timeout}` 作为最终 identity，timeout 变化也不能遮蔽逻辑 request 的其他输入。

## 5. 单 Agent single in-flight 状态机

```text
Idle
  -> Preparing
  -> InFlight(request identity)
     -> Candidate
        -> AwaitingReceipt (candidate only)
           -> Completed | Rejected | Cancelled | Stale | Failed | Pending
     -> Wait -> Pending
     -> Rejected | Cancelled
```

实现约束：

- `InFlight` 下只能有一个活动 adapter attempt；retry 必须先结束上一次 attempt；
- single-flight mutex 的主键是 `agent_subject`：同一 Agent 同时至多一个 active session 和一个 active turn；session id 只用于分区，不能放宽该全局互斥。新 session 只有在旧 session `draining/closed` 或已完成显式 recovery handoff 后才能激活；
- `AwaitingReceipt` 下不能开启下一 turn，避免未确认 action 与下一次 observation 交叠；
- 新 observation 只能 coalesce 为下一 turn 的输入，不能覆盖 active request；若 authority head 已变化，active turn 进入 `Stale`；
- 不同 Agent 的 state machine 可并行，但所有队列、cache、feedback buffer 必须以 session/subject 分区；
- 任何重入都返回 `agent_busy` 或 `protocol_violation`，不会向 provider 发第二次逻辑调用；
- world 是否在 `InFlight` 期间推进属于 runtime scheduler 合同。Harness 只保持 turn identity 和 pending outcome，不假设同步阻塞 world。

### 5.1 Session/Turn lifecycle mapping

Harness 的 cognition 状态必须能映射到 Runtime 的生命周期，但不取代 Runtime 的真相：

| Harness state | Runtime state/disposition | 处理语义 |
| --- | --- | --- |
| `Preparing` | `queued` / `running` | Harness 已分配 session/turn/request IDs；Runtime 验证并持久化 envelope 后才允许运行。 |
| `InFlight` | `waiting_provider` | 一个逻辑 request 只能有一个活动 provider attempt；Runtime lease 仍有效。 |
| `Candidate` | `response_recorded` / `ready_to_submit` | Harness 已规范化 candidate；Runtime 仍须验证 authority、precondition 和 envelope。 |
| `AwaitingReceipt` | `accepted` / `rejected_*` / `failed_*` / `recovery_pending` | 等待 Runtime authoritative disposition；不得以 provider transcript 当 receipt。 |
| `Wait` | `pending` | Agent 选择无副作用等待；保留因果 identity，等待 Runtime wake/recovery；不启动同一 Agent 的下一逻辑 turn。 |
| `Completed` | `committed` | 只在收到匹配的 Runtime receipt 后结束，并按 policy 提交 memory/continuation。 |
| `Stale` | `rejected_stale` / `recovery_pending` | head、lease、precondition 或 digest 失效；丢弃 candidate，清除不兼容 continuation。 |
| `Failed` | `failed_*` | 终止当前逻辑 request；保留 bounded failure trace，禁止 heuristic fallback。 |
| `Rejected` | `rejected_*` | Runtime 拒绝或 Harness 规范化拒绝；释放 single-flight、清除不可复用 continuation；迟到 response 只能进入 `stale/expired` diagnostic。 |
| `Cancelled` | `cancelled -> rejected` with `reject_reason=cancelled` | 终止 turn、释放 single-flight、清除 continuation；迟到 response 按下方 `late_response_after_cancel` 规则进入 `stale/expired` diagnostic。 |
| `Pending` | `scheduler_backpressure` / `recovery_pending` | 保留同一 identity 等待 capacity/recovery；不调用 provider、不开新逻辑 turn。 |

`retry_seq` 是 semantic retry 序号：新的逻辑 turn/request 必须递增并建立因果引用；同一逻辑
request 的 transport retry 只递增 `transport_attempt`，不改 `request_digest` 或
`provider_invocation_key`。Harness 独占分配 session/turn/request cognition IDs；Runtime 验证、持久化
它们并分配 journal/action/envelope/receipt/wake IDs。

Runtime lease expiry 必须把 active turn 变为 `pending` 或 `failed_*`，关闭 Harness 的 in-flight
占用并清除不可复用 continuation；过期后到达的响应只能成为 `stale/expired` diagnostic 或匹配
既有 receipt，不能重新打开 turn。崩溃恢复在 journal 证明同一逻辑 request 时保留 session/turn/request
identity；无法消解的冲突进入 `recovery_pending`。Session 只允许 `active -> draining -> closed`，
`closed` 后不接受 cognition 或 feedback。

Runtime 的 `scheduler_backpressure` 必须释放本地 attempt lease、保留 bounded pending record，
并在 capacity/recovery wake 后只恢复原 identity；`cancelled`、rejected 或 failed 均清除不可复用
continuation。Runtime 的 canonical feedback status 仍只有 `committed/rejected/failed/pending`，
因此 `cancelled` 必须以 rejected reason 映射给 Harness，而不是泄漏为第五个 status。

状态 precedence 固定为：`Wait` 只表示 Agent 的无副作用决定；Runtime
`scheduler_backpressure` 或 `recovery_pending` 一旦产生，Harness 进入 `Pending` 并覆盖本地
Wait 标签。`Pending` 不是第二种 feedback status，也不得与 `Wait` 合并为重复的 state row。

取消后的响应映射固定为四层：Runtime 内部 disposition/code 为
`late_response_after_cancel`；若 Runtime 生成 FeedbackEnvelope，字段为
`status=rejected`、`reject_reason=late_response_after_cancel`，且无 receipt/effect；Harness
将其映射为已关闭 turn 的 `stale`（若 lease/retention 已过期则为 `expired`）bounded
diagnostic，保留原 `feedback_id/feedback_seq` 和 reason。该诊断不是新的 AgentTurn status，
不得把 `cancelled` turn 重新打开，也不得进入 prompt、memory 或 continuation。

## 6. 统一 adapter lifecycle

所有 provider adapter 实现同一抽象生命周期：

```text
open_session(runtime binding)
  -> prepare_turn(observation, memory, goal, continuation)
  -> issue(request)              // one active attempt
  -> normalize(response | error)
  -> handoff(candidate)          // Runtime authority
  -> accept_feedback(feedback)   // correlated only
  -> close_turn / retry / stale / cancel
```

Builtin 的 Responses/tool-only prompt 和 ProviderBacked 的 external session/tool protocol 只能改变 `issue/normalize` 的内部实现。两者不可改变：

- request identity、digest 和 timeout/retry semantics；
- candidate 必须回到 Runtime validation；
- memory intent 的 policy、continuation 的 bounds 和反馈 correlation；
- error code、safe Wait/Rejected 和 trace completeness fields。

Adapter 不得自己选择 subject/grant/audience/nonce/cost quote，也不得把 provider tool call 当成已提交 action。旧的显式 `with_capability_context` 可作为 fixture seam；只有 runtime-owned per-turn injection 才能进入 production target。

Provider trace bounds are fixed for the target contract: at most 64 transcript entries and 64
tool-trace entries per turn, each entry at most 2 KiB and 1 KiB respectively, input/output
summaries at most 1 KiB each, upstream trace at most 4 KiB, and total canonical trace payload at
most 32 KiB. Non-authority trace overflow produces `trace_payload_too_large`, drops only the
diagnostic artifact, and never changes a valid candidate's Runtime disposition; authority-bearing
overflow is a protocol violation. Redaction uses the fixed `<redacted>` marker for credential,
token, authorization, private-key and path fields; unrecognized sensitive values fail closed.
There is no silent truncation. Required fixtures cover each overflow and redaction case.

## 7. Feedback routing 与污染防线

Harness 为每个 session 建立 bounded feedback partition：

```text
feedback_partition[(agent_subject, agent_session_id)]
  -> ordered { feedback_id, feedback_seq, agent_turn_id, decision_request_id,
               candidate_action_id, runtime_receipt_id }
```

路由前按下列顺序检查：

1. session/subject match；
2. turn/request match 当前 active 或 recently completed turn；
3. action/receipt lineage match；
4. sequence 不重复且未越过 retention boundary；
5. summary size、redaction 和 sensitivity policy 通过。

`feedback_seq` 必须按 Runtime session sequence 单调消费：gap 只进入 bounded hold buffer，
不进入 prompt；缺口超过 retention boundary 返回 `feedback_sequence_gap`，关闭当前 feedback
projection 但不按到达时间重排。相同 sequence+digest 是幂等 duplicate；相同 sequence+不同
digest 是 `feedback_identity_collision`，fail closed。

检查失败的 feedback 只生成 bounded diagnostic (`late_feedback`、`unknown_feedback`、`cross_agent_feedback` 或 `duplicate_feedback`)。它不会进入 prompt、memory、continuation，也不会被全局 recent buffer 复用。多 Agent 交错 fixture 必须验证在相同到达顺序和不同到达顺序下，双方 context 仍只包含自己的反馈。

未匹配的 feedback 本身不会结束活动 turn；活动 turn 只能由匹配的 Runtime `pending`/`failed_*`
disposition 或 lease expiry 终结。终结后迟到响应走 `stale/expired` 或既有 receipt 路径，并保持
bounded diagnostic，不得重新注入已关闭 turn。

旧 `FeedbackEnvelope` v1 的 field-level mapping 固定为：`action_id -> candidate_action_id`；
`success=true` 只有在 Runtime 提供 committed receipt 时才映射为 `status=committed`；
`success=false` 必须携带 Runtime disposition 才能映射为 `rejected`/`failed`/`pending`，
否则 fail closed 为 `legacy_feedback_ambiguous`；`reject_reason` 原样作为稳定 reason，
`emitted_events/world_delta_summary` 只进入 bounded、redacted projection。旧 DTO 没有 session/
turn/request/digest/receipt correlation 时只能进入 `legacy_v1` compatibility lane，不能关闭
target turn 或进入 target memory/continuation。

Feedback projection 的上限固定为：canonical envelope 总计最多 16 KiB、`emitted_events`
最多 32 项、`committed_event_summary` 最多 512 UTF-8 bytes、`world_delta_summary`
最多 1024 UTF-8 bytes。Runtime 必须在发送前满足上限；Harness 收到超限 projection 时
记录 `feedback_projection_too_large`，只保留已验证的 identity/status/reason，不把超限摘要
注入 context/memory，并仍按 authoritative terminal disposition 关闭 turn。redaction 固定
移除 credential/token/authorization/private-key/path 字段并替换为 `<redacted>`；无法安全
识别的敏感 payload 直接拒绝 projection，不做静默截断。超限、敏感字段和 gap/乱序都必须
有负例 fixture。

## 8. Memory policy 与 provenance

### 8.1 Retrieval

Harness 在打开 turn 时冻结 `MemoryContextSnapshot`：本地权威 memory 的 selected entries、revision、scope 和 digest。provider 读取的是这个不可变 projection；它不能在 request 内替换 memory backend 或隐式读其他 Agent。

无 memory entry 时使用 schema-defined empty snapshot（`revision=0`、空 entries、显式 scope
和其 canonical digest）；不得用缺字段、JSON `null` 或空字符串替代。空 snapshot 不阻止
decision，但任何 write intent 仍须通过本节 policy 和 committed outcome gate。

### 8.2 Write intent

处理顺序为：

```text
provider intent
  -> source/session/turn check
  -> scope/size/tag/content policy
  -> intent digest + dedupe
  -> wait for authoritative Runtime outcome
  -> local memory policy commit or reject
```

P0 只允许 Agent-private、bounded、可脱敏的 intent policy；共享世界/玩家可见/长期事实语义需要另行授权。写入必须记录 source turn、request digest、Runtime receipt lineage 和 provenance。没有 committed receipt 的 intent（包括 Wait、timeout、malformed、stale、deny）不得进入 authoritative memory；可保留 diagnostic discard reason。

`intent_digest` 相同的 retry 只允许一次 commit；scope、size、敏感度或 provenance 不符合 policy 的 intent 被拒绝且不影响其他 intent。Runtime/journal owner 负责持久化 exactly-once 和 replay proof，本设计不伪造该证明。

### 8.3 MemoryWriteIntentPolicyV1

target memory candidate 的 wire schema 固定为：

```text
MemoryWriteIntentV1 {
  schema_version: 1,
  scope: turn_private | session_private | agent_private_long_term,
  summary?,                         // optional; explicit presence encoding
  tags: string[],                   // omitted tags encode as explicit empty list
}
```

`summary?` 的存在性和值由 Harness 规范化；`intent_digest` 是 Harness 根据下方 policy
计算的 derived artifact，不是 provider 可自报的 authority。现有 inner
`MemoryWriteIntent { scope: String, summary: String, tags: Vec<String> }` 保持不变，仅能经
显式选定的 legacy route/adapter 映射；target lane 不按旧 DTO shape 猜测版本。当前 HTTP wire
不定义 `compatibility_lane` 字段。

P0 的 memory write gate 是确定性的 `MemoryWriteIntentPolicyV1`。scope registry 保留
`agent_private_long_term` 作为版本化保留值，但默认 P0 allowlist 只有
`turn_private | session_private`；long-term scope 默认返回 `memory_scope_denied`，并延后到
P2/独立 memory authority，不得作为 P0 proven 行为：

- `scope` wire enum 为 `turn_private | session_private | agent_private_long_term`；P0 默认只允许 `turn_private | session_private`，共享世界、玩家可见和跨 Agent scope 均拒绝。
- 每 turn 最多 8 个 intent；每个 `summary` 最多 512 UTF-8 bytes；每 intent 最多 8 个 tags；每个 tag 最多 64 bytes；全部 intents 的 canonical bytes 总计最多 4096 bytes。
- `summary?` 是可选字段：省略时以显式 `present=false` 编码；若出现则先做 Unicode NFC 和 trim，且必须非空、无 NUL/control 字符。`tags` 可以是显式空列表或按 schema 省略（省略编码固定等同于 canonical empty list）；每个 tag 先 NFC/trim 后必须非空、无 NUL/control 字符。tags 以 canonical UTF-8 bytes 排序并去重；其他 list 不得套用该规则。
- `intent_digest = H_v1("oasis7.cognition.memory-write-intent.v1", canonical(payload))`；payload 固定为 policy version、agent/session/turn identity、`request_digest`、scope、summary presence/value、canonical tags 与 host-assigned provenance，并排除 digest 自身、transport attempt、raw transcript 和日志顺序。adapter 不得用空 summary、JSON `null` 或缺失字段替代显式的 summary presence encoding。
- 兼容 lane 可把旧 `scope=short_term` 显式映射为 `turn_private` 并记录 `memory_scope_alias_used`；旧 DTO 的 summary 必须为非空字符串。target lane 不接受该 scope alias，必须返回 `memory_scope_denied`；空 tags 仍允许，但空 tag element 必须返回 `memory_tag_invalid`。`agent_private_long_term` 只有显式 future authority/feature gate 才可开启，仍不属于 P0 target。
- 稳定拒绝码为：`memory_source_mismatch`、`memory_scope_denied`、`memory_intent_count_exceeded`、`memory_summary_too_large`、`memory_tag_invalid`、`memory_tag_count_exceeded`、`memory_payload_too_large`、`memory_sensitive_content`、`memory_no_committed_outcome`、`memory_intent_duplicate`、`memory_digest_mismatch`。
- provider 不得自称 `runtime_fact` 或授予 provenance；只有匹配的 Runtime `committed` receipt 才可授予该 intent 的 authoritative provenance。`rejected`（包括 `reject_reason=no_effect`）、`failed`、`pending`、stale 或 lease expiry 均不得写入 authoritative memory。

## 9. GoalSnapshot 与 bounded continuation

### 9.1 P0 GoalSnapshot

```text
GoalSnapshot {
  revision,
  short_term_summary,
  long_term_summary,
  blocked_reason?,
  provenance: harness_projection,
  digest,
}
```

它是兼容性 projection，不是完整 GoalGraph。projection precedence 固定为 trusted host
Agent config/runtime goal projection > legacy provider `goal_summary/blocked_reason`；provider
字段不能覆盖已存在的 host 值。各摘要先 NFC+trim，`short_term_summary`、`long_term_summary`
各最多 512 UTF-8 bytes，`blocked_reason` 最多 256 bytes；超限返回
`goal_snapshot_too_large`，不得静默截断。无值使用 `revision=0`、空摘要、无
`blocked_reason` 的显式 empty snapshot。provider 可以返回 `goal_update_intent`，但 world goal
completion、权威 priority、玩家承诺和跨 Agent goal 关系需要产品/runtime authority。

### 9.2 Continuation

```text
ContinuationState {
  origin_turn_id,
  origin_request_digest,
  action_or_plan_kind,
  remaining_budget: { unit: steps | ticks, value: u64 },
  baseline_observation_digest,
  goal_digest,
  policy_digest,
  precondition_summary,
  valid_until_tick?,
}
```

Harness 提交 `ContinuationProposalV1`，Runtime 将其验证并投影为 `AgentContinuation`：

```text
ContinuationProposalV1 { // Harness-owned policy proposal
  schema_version,
  continuation_proposal_id, // Harness allocated cognition ID
  world_id,
  agent_id, // Harness subject; Runtime validates current binding
  agent_session_id,
  agent_turn_id,
  decision_request_id,
  origin_turn_id,
  origin_request_digest,
  action_or_plan_kind,
  action_or_envelope_digest?,
  remaining_budget: { unit: steps | ticks, value: u64 },
  baseline_observation_digest,
  goal_digest,
  policy_digest,
  policy_revision,
  precondition_summary,
  precondition_digest,
  wake_conditions: WakeConditionV1[],
  valid_until_tick?,
  source,
  proposal_digest,
}

AgentContinuation { // Runtime-owned durable projection
  continuation_id, // Runtime allocated schedule ID
  wake_id, // Runtime allocated schedule ID
  world_id,
  branch_id,
  finality_epoch,
  finality_block_hash?,
  finality_status,
  reorg_epoch,
  runtime_manifest_hash,
  agent_id,
  agent_session_id,
  agent_turn_id,
  decision_request_id,
  origin_turn_id,
  origin_request_digest,
  continuation_proposal_id,
  proposal_digest,
  action_or_envelope_digest?,
  wake_conditions: WakeConditionV1[],
  next_wake_tick?,
  remaining_budget: { unit: steps | ticks, value: u64 },
  valid_until_tick?,
  precondition_digest,
  wake_seq,
  status,
  terminal_disposition?,
}
```

Harness 独占 proposal policy、bounds 和 `continuation_proposal_id`；Runtime 分配并持久化
`continuation_id`、`wake_id`、`wake_seq`、`next_wake_tick` 等 schedule fields，且依据 paired
runtime disposition-to-feedback mapping 返回终态。以上 `ContinuationProposalV1` 是唯一 wire
schema，不存在 `proposal_id`、`proposer_agent_id`、`request_digest`、`action_digest`、
`remaining_ticks` 或 `expires_at_boundary` 等 wire alias。Runtime 由 wake_conditions 确定
`next_wake_tick`；当 remaining_budget.unit 为 steps 时不得偷换成 tick，Runtime durable record
保留 unit/value 和 proposal digest。origin_request_digest 只作 lineage 校验，不得冒充 action
digest。Runtime ID 不能由 Harness 或 provider 伪造。

`FinalityBindingV1` 是 Runtime 与 Harness 共享的 branch/finality binding 子结构，不向
`ContinuationProposalV1` 添加第二套 wire 字段；proposal 的 `world_id` 由 Runtime 校验，
projection 与 continuation context 携带同一组 Runtime world binding：

```text
FinalityBindingV1 {
  schema_version, branch_id, finality_epoch: u64,
  finality_block_hash?: Digest32, finality_status: pending | verified | reorged | suspended,
  reorg_epoch: u64
}
finality_binding_digest = H_v1(
  "oasis7.runtime.finality-binding.v1", canonical(FinalityBindingV1))
```

`AgentContinuation` 的 `world_id/branch_id/finality_epoch/finality_block_hash/finality_status/
reorg_epoch/runtime_manifest_hash` 必须
来自同一已验证 binding；reorg/finality/world drift 进入 Runtime rejection，而不是由 Harness
或 provider 修补。该形状与 paired Runtime continuation projection 保持字段名和语义一致。

Digest 规则固定为：

```text
proposal_digest = H_v1(
  "oasis7.cognition.continuation-proposal.v1",
  canonical(ContinuationProposalV1 without proposal_digest),
)
continuation_digest = H_v1(
  "oasis7.cognition.continuation-context.v1",
  canonical({ continuation_proposal_id?, proposal_digest?, continuation_id?, wake_id?, wake_seq?,
              world_id?, branch_id?, finality_epoch?, finality_block_hash?, finality_status?,
              reorg_epoch?, runtime_manifest_hash?, remaining_budget?, valid_until_tick?,
              status?: ContinuationStatusV1, continuation_status_digest?, terminal_disposition? }),
)
```

`ContinuationStatusV1`、允许迁移和 `continuation_status_digest` 的字段/domain 完全采用 paired
Runtime contract；Harness 只消费该 Runtime-owned projection，不得生成或改写 status digest。

两者使用共享 BLAKE3-256/RFC 8949 deterministic CBOR registry；digest 字段自身排除在输入
外。没有 continuation 时仍编码上述 context 的全 absent canonical object，作为唯一 empty
sentinel。Runtime 必须验证 proposal_digest，随后把 Runtime-owned schedule correlation 投影为
continuation_digest；adapter 不得用 raw JSON、空字符串或本地 null 替代。

Continuation 的 policy、proposal、bounds、advance/invalidate 语义由 Harness 拥有；它只能作为下一 turn 的 bounded hint。Runtime 拥有 durable schedule、wake、crash recovery 和最终执行时机，并在每次接受 candidate 时检查 live action semantics/preconditions。Harness 每次继续前检查 observation/goal/policy digest 和 remaining budget。receipt reject、stale、timeout、expiry、reorg/finality invalidation 或 changed digest 清除 continuation；不得自动换成相似 action，不得绕过 provider 或 Runtime。Builtin 与 ProviderBacked 共享该状态机。

GoalGraph、belief memory、长期 preference correction 和 player-facing goal explanation deferred beyond P0；它们不能通过扩展 `GoalSnapshot` 字符串被隐式宣称已交付。

## 10. Failure normalization

Adapter 将 transport/model-specific failures 映射到稳定 Harness errors：

| normalized error | 触发 | 行为 |
| --- | --- | --- |
| `request_identity_collision` | same logical request id, different digest | fail closed；不覆盖 dedupe entry |
| `request_stale` | runtime head/context/policy/capability digest 变化 | 结束旧 turn，`NeedsRediscovery`，无副作用 |
| `provider_unavailable` | timeout/connection/provider unavailable after budget | bounded retry then `Wait` |
| `invalid_decision` | malformed/unknown/disallowed output | `ActionRejected`，不 heuristic fallback |
| `budget_exhausted` | timeout/token/repair cap reached | `Wait`，保留 bounded trace |
| `response_identity_mismatch` | response 未回显同一 identity/digest | `ProtocolViolation`，丢弃 candidate/intents |
| `feedback_not_correlated` | late/unknown/cross-agent/duplicate | diagnostic-only；保持 active state，直到 Runtime pending/failed 或 lease expiry 终结 |
| `late_response_after_cancel` | Runtime 已将 turn 置为 cancelled 后仍收到 response | 映射为 `stale` 或 `expired` diagnostic；不重新打开 turn、不进入 context/memory/continuation |
| `memory_intent_rejected` | scope/size/provenance/dedupe policy failure | 丢弃该 intent，保留 rejection trace |
| `runtime_denied` | Runtime authoritative validation reject | 使用拒绝 receipt，清除不兼容 continuation |

所有失败都必须是 deterministic、可诊断、无未授权副作用。禁止以“模型大概理解了”作为重试/解析/动作 fallback 的理由。

## 11. Rollout design

### P0.1 identity/correlation proof

P0.1 冻结 outer context、shared `request_digest`、`provider_invocation_key`、per-agent global single-flight、feedback partition、memory intent gate、GoalSnapshot/continuation projection 和稳定错误码。P0.1 required fixtures：digest golden vectors、same-retry/new-turn/collision、single-flight reentrancy across sessions、A/B feedback interleave、memory intent rejection/commit gate、legacy module/feedback mapping、bounds/redaction/overflow/cancelled/backpressure、Builtin/ProviderBacked MockProvider contract parity。

### P0.2 async actor + action MVCC coupled proof

P0.2 要求 Harness async AgentActor/single-flight 与 Runtime action MVCC/stale/precondition disposition 共同形成可验证 seam；scheduler/MVCC 具体实现仍由 paired runtime docs 冻结。本阶段未通过前不得声称 production async cognition。

### P0.3 minimum journal/recovery proof

P0.3 要求 active turn/request、provider invocation result、Runtime disposition 和 memory intent disposition 在 crash/retry/reconnect 后可恢复且不重复副作用；journal/recovery schema、存储和 replay 实现仍由 paired runtime docs 冻结。P0.1、P0.2、P0.3 全部 required 通过后，才允许 production async claim。

### P1 provider convergence / durable continuation

P1 将两个 provider runner 接入同一 Harness lifecycle，并把 continuation policy/proposal 与 Runtime durable schedule/wake/recovery seam 接通。P1 full fixtures：provider disconnect/retry、Runtime deny/stale、receipt correlation、restore without double intent、continuation invalidation 和 bounded trace/cost diagnostics。

### P2 advanced goals/memory/economy / pilot

P2 在 P0/P1 proof 后执行一个低频、低破坏性 NPC/未知 governed command fixture，再进行固定 epoch 的多样本评测。Cognition economy、GoalGraph、belief memory 和共享/玩家可见 memory 均需额外 authority 与 gates；未通过单场景负例或成本/稳定性门槛时保持 experimental。

## 12. Verification matrix 与 observability

| 证据层 | 主要断言 | 关键 artifact | 禁止外推 |
| --- | --- | --- | --- |
| required | schema/identity/错误/隔离契约 | canonical bytes、digest、request/response/feedback fixtures | 不能证明真实 provider 或 world commit |
| required | legacy lane、trace/feedback bounds、redaction、cancelled/backpressure cleanup | explicit lane markers、bounded projections、terminal disposition、late-response fixtures | 不能证明真实 provider 或 target parity |
| full | Runtime causal loop/replay/no duplicate | head-bound receipt、stale/deny/retry/restore logs、journal proof | 不能证明产品 parity 或 economy |
| rollout | 行为/延迟/成本/稳定性 | fixed scenario/profile/provider/adapter/protocol epoch，per-scenario artifacts 和 repeated samples | 均值不能掩盖单场景 failure |
| proven | 同一 epoch 的成功+负例闭环 | artifact digest、identity binding、副作用检查、reviewed fixture manifest | 未覆盖 provider/module/goal 不可继承 proven |

最低诊断字段：session/turn/request ids、request/response digest、provider/adapter/protocol version、base head、outcome/error code、attempt count、latency/token/cost diagnostics、feedback lineage、memory intent disposition、continuation disposition。token/latency/cost 在 P0 只是 diagnostics/evaluation inputs；不代表资源扣账或发布证据。

可执行入口固定为：文档阶段运行 `./scripts/doc-governance-check.sh`；P0.1 实现阶段运行
`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_provider_local_bridge -- decision_identity`
与 `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib provider_loopback_adapter`；集成阶段运行
`env -u RUSTC_WRAPPER cargo test -p oasis7 --lib agent_cognition_mvcc`（P0.2 paired runtime）与
`env -u RUSTC_WRAPPER cargo test -p oasis7 --lib agent_cognition_recovery`（P0.3 paired runtime）；
`./scripts/ci-tests.sh required`，完整多 Agent/retry/restore/parity 证明运行
`./scripts/ci-tests.sh full`。focused filter 必须由实现切片建立并输出 immutable golden
fixture、canonical bytes/digest/key、provider invocation count 与 feedback partition；命令
不存在或未执行时仍是 target，不是通过。

## 13. Paired specialist follow-ups

- `runtime_engineer`：scheduler 非阻塞语义、candidate/action MVCC、stale/precondition、receipt/journal/replay 和恢复的权威合同；Harness 只接收其 binding。
- `qa_engineer`：required/full/rollout fixture 的 deterministic assertions、交错并发、cross-talk、stale/replay/no-duplicate side-effect 和 failure matrix。
- cognition economy authority：预算 reserve/settle、计费与 receipt，明确晚于 P0。
- gameplay/product/Agent authority：GoalGraph、belief memory、偏好、共享/玩家可见 memory 和目标完成语义。
- viewer/runtime-live owners：trace delivery 与玩家可见诊断；不得把 Viewer transcript 当成 cognition journal。

任何 follow-up 都必须回到同一 task/worktree/PR 主链；本设计不创建第二份 mutable task truth。
