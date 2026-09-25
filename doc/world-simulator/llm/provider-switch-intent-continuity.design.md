# Provider 切换窗口中的 Agent 意图连续性系统设计

- 设计 ID：DES-PSW
- 状态：active target contract；不代表实现或 provider readiness
- Owner role：agent_engineer
- 适用范围：world-simulator Continuous Agent Harness、provider adapter 与 provider/lane 选择之间的切换和恢复
- 上游产品要求：[AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002)
- 专业依赖：[Continuous Agent Harness](continuous-agent-harness.design.md)、[Agent cognition lifecycle](../../world-runtime/runtime/agent-cognition-lifecycle.design.md)、[indirect-control runtime authority](../../world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md)
- 审读基线：结合上游文档的现行 target contract 阅读；实现状态、候选版本和任务证据由关联 GitHub Task UID issue 维护
- Last reviewed：2026-09-25

本文只冻结 provider/lane 切换时的 Agent 路由绑定和意图 lineage。它不声明任何 provider 当前受支持、可用、默认或已就绪。

## 1. 问题、目标与非目标

provider 或 execution lane 变更发生在一个 Agent session 或 cognition turn 尚未收口时，若当前路由被覆盖，旧 response 可能被新 adapter 当成新输入处理；若断线后用新 provider 重建请求，则可能重复产生意图，或让玩家误以为原请求已取消、失败或成功。

目标是在 provider/lane 选择变化和进程重连下保持清晰的边界：旧 turn 沿原绑定和身份收口；新绑定只服务新建的语义 turn；Runtime 是已提交意图的唯一权威裁决者。目标是可评测的设计合同，不是当前实现结论。

非目标：重新定义 action schema、provider 支持矩阵、lane 的玩法含义、Runtime intent/receipt schema、撤回或替换协议、Viewer 布局、parity 阈值、默认 provider 或发布准入。

## 2. 上游约束与相关角色

`producer_system_designer` 拥有切换窗口的产品承诺和适用范围；`agent_engineer` 拥有 provider/lane 路由快照、Agent turn 与 semantic replan；`runtime_engineer` 拥有已提交请求的接受/拒绝、幂等、receipt、持久化和恢复；`qa_engineer` 证明切换与恢复组合；`viewer_engineer` 投影候选、待决和终态。

产品 PRD 要求切换只影响后续新意图、候选不得自动交给新 provider 重试、pending 不因切换或重连改变语义，且只有 committed receipt 表达世界后果。意图 lineage、世界效果和 UI 投影仍由各自专业 authority 决定。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | active session 的 provider/lane 路由快照不可被切换追溯改写；后续新 turn 使用新绑定，且绑定差异参与该 turn 的请求身份。 | [DES-PSW-001](provider-switch-intent-continuity.design.md#des-psw-001) | Agent Harness owner；依赖 [Continuous Agent Harness](continuous-agent-harness.design.md) 的 session、turn 和 digest 合同。 | 不指定 provider 矩阵、支持状态或对外 UI 字段。 |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | pre-switch candidate 保留原 turn 来源，不自动交给新 provider；提交仍须符合原 turn 既有授权和 Runtime 当前校验。 | [DES-PSW-002](provider-switch-intent-continuity.design.md#des-psw-002) | Agent Harness owner；依赖 [DES-WR-IA-001](../../world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-001) 的接受/生效边界。 | 不定义 AgentIntentV2、撤回或 replacement schema。 |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | provider failure、重连或 receipt 丢失时，只能按原绑定和原 identity 恢复；新 provider 不得承接或重发旧请求。 | [DES-PSW-003](provider-switch-intent-continuity.design.md#des-psw-003) | Agent Harness 与 Runtime；依赖 [DES-WR-IA-004](../../world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-004) 和 [DES-WR-IA-006](../../world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-006)。 | 不规定 Runtime journal、receipt schema 或保留期。 |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | required-tier 覆盖身份和状态负例；full-tier 在同一 fixture/profile 上覆盖真实 provider、Runtime receipt/journal 与 restart/reconnect。 | [DES-PSW-004](provider-switch-intent-continuity.design.md#des-psw-004) | QA 与 Agent；full-tier 的 authoritative outcome 需要 Runtime evidence。 | 本设计不报告任何 candidate 的通过、parity 或 release 结论。 |

### 2.2 当前状态与目标差距

| 对象/能力 | 当前状态（基线） | 目标状态 | 差距/假设 | 证据或 owner |
| --- | --- | --- | --- | --- |
| Request identity | Continuous Harness target contract 已把 provider config/profile revision、adapter protocol 与认知上下文纳入 request digest；transport retry 与 semantic retry 分开。 | 每个 turn 的完整 provider/lane 绑定都不可变，且可从 request identity 复原。 | 现有材料未明确 lane 是否已进入绑定 digest，也未定义配置切换时的 session drain 语义。 | [Continuous Agent Harness design](continuous-agent-harness.design.md)；实现覆盖须由当前 task evidence 证明。 |
| Runtime request recovery | Runtime target contract 定义 request identity、幂等和 receipt/recovery owner。 | 重连只恢复原 authority request 的 pending/terminal disposition。 | provider/lane cutover 本身不能成为 Runtime 的取消或重放信号。 | [Agent cognition lifecycle design](../../world-runtime/runtime/agent-cognition-lifecycle.design.md)。 |
| Player-facing parity | 产品 AC 定义 candidate、pending、committed 与 rejected 的玩家边界。 | 切换前后身份与实际 Runtime 结果可追溯，且状态不会因换路由被改写。 | Viewer 与真实 provider/restart 样例未由本文证明。 | Product PRD、Viewer owner、QA evidence。 |

文档中 `target` 只描述未来合同。没有固定实现 candidate 和对应验证证据时，不能报告为 `current` 或 `proven`。

## 3. 决策概览

一个活动 AgentSession 绑定一个不可变 provider profile revision 与 execution lane selection。provider/lane 配置改变时，Harness 将该 session 置为 draining；已创建 turn 继续引用旧快照。旧活动 turn 得到原 provider 的有界响应或明确失败、并完成任何已提交 Runtime request 的权威对账后，Harness 才能基于新快照开启新 session/turn。

若当前 request context 无法把有效 provider/lane selection 唯一绑定到该 turn 的 canonical identity，则不能声称满足本合同。应先演进 request-context/digest version，再让新 turn 使用该格式；不得在旧 turn 上补写 lane 或 profile 字段。

## 4. 边界与结构

~~~text
player / host route selection
  -> Agent Harness freezes provider profile + execution lane for session/turn
  -> original provider adapter call and response identity check
  -> candidate remains advisory data
  -> Runtime validates any submitted request and owns disposition / receipt / effect
  -> Harness recovers matching feedback before forming a later turn
~~~

- Harness owns the cognition session/turn binding, candidate lineage, adapter route and decision to create a fresh semantic replan.
- Provider adapter owns transport and provider-specific invocation. It cannot change request identity or turn an old candidate into a new request.
- Runtime owns only requests already submitted to world authority. It validates, deduplicates, commits or rejects, and supplies authoritative receipt/recovery facts.
- `player_parity` and `headless_agent` remain execution lanes, not access modes. The lane selection does not alter permissions, action semantics, world authority or product access promises.
- Runtime may correlate a submitted request to its Agent turn, but provider/lane metadata is Agent-side provenance. This design adds no Runtime enum or schema field.

## 5. 关键运行流程

### 5.1 正常切换

1. The host resolves a provider profile and lane before starting a session. The binding is immutable for that session and every turn it issues.
2. On a provider/lane change, the Harness marks the old session draining. It does not edit the old request context, digest, provider invocation key, candidate, Runtime request or receipt reference.
3. An active provider invocation is allowed to finish on the old route or reach its existing bounded timeout. A late response is usable only if all old turn identity and digest fields match.
4. When the old turn is terminal under its existing Agent/Runtime contract, a new session captures the selected profile/lane revision. Its first request is a fresh semantic turn with new identity and a new digest.

### 5.2 Candidate at cutover

A candidate created before cutover stays attached to its old turn. The cutover neither sends it to the new provider nor authorizes it for world execution. If the existing Agent contract permits that exact candidate to proceed, it may be submitted once through its original lineage and still must pass current Runtime validation. If a new provider decision is needed, the Agent retires the prior candidate according to its existing contract and creates a fresh semantic request with a causal reference; it must not copy the old candidate payload into the new provider request as a retry.

If the candidate has already been submitted, it is no longer a provider-routing concern. Reconnect and observation use the same Runtime request identity and let the same authority chain return its existing pending or terminal disposition. A new recommendation does not replace, cancel or supersede it without an independently supported replacement path.

### 5.3 Reconnect and lost acknowledgement

- Reconnect without a binding change resumes the same Agent session and request identity. A transport retry may increment `transport_attempt` only under the existing same-key/same-digest adapter contract.
- If provider response delivery was uncertain, retrieve or idempotently retry only through the original provider binding. If that provider cannot prove same-key deduplication or retrieve its response, do not replay through another provider; wait for the old turn's explicit bounded failure/recovery path.
- If Runtime submission may have succeeded but the acknowledgement or receipt was lost, query/retry that exact Runtime request identity and digest. Same identity plus same digest recovers the original disposition; same identity plus a different digest fails closed.
- Runtime `pending` remains pending until Runtime provides its terminal disposition. Provider switch, reconnect, timeout observation or client-side transport cancellation alone cannot make it cancelled, rejected, committed or failed.
- A fresh post-switch semantic replan has a new turn/request identity and causal link to the prior result. It remains a candidate until independently validated by Runtime; Runtime must not infer retry, merge, shared authorization or deduplication from matching action text.

## 6. 接口与数据合同

### 6.1 Immutable routing binding

The logical binding consists of the selected provider configuration reference, its immutable profile revision, adapter protocol version, execution-lane selection/revision, and the originating session/turn identity. Credentials, raw endpoint secrets, mutable display names and transport attempt counters are not binding identity.

The binding must be represented by the existing request context fields or an explicitly versioned additive context that participates in canonical request digest and provider invocation key derivation. The design does not require a particular JSON/Rust field layout. If an existing lane selection cannot be recovered from the bound profile or included in that digest, that context version is ineligible for this target behavior; do not silently add an unhashed side channel.

| 接口/条款 | producer → consumer | identity/version | ordering/idempotency | success/error | compatibility |
| --- | --- | --- | --- | --- | --- |
| Session route snapshot | trusted Agent host → Harness/adapter | session id plus selected profile revision and lane revision | frozen before first turn; no in-place mutation | unknown/invalid binding fails closed before provider invocation | legacy context without complete binding remains compatibility-only and cannot claim this contract |
| Provider decision call | Harness → selected adapter | existing session/turn/request ids, request digest, provider invocation key and adapter protocol version | transport retry preserves identity/digest; semantic replan creates new turn/request | response must echo/match the old context; late or mismatched response is rejected for that turn | existing DTO remains inner decision payload; no second action schema |
| Runtime request recovery | Harness → same Runtime authority | Runtime-owned request/intent identity and digest, correlated to Agent turn | retry same identity and digest only; conflicting digest is rejected | authoritative pending/terminal disposition and receipt determine world result | Runtime schema and status mapping remain owned by Runtime design |

No cross-provider fallback is implicit. A fallback after failure is a new semantic turn on the newly selected provider, not transport retry. Any profile or lane change affecting request semantics changes the digest identity and requires a new request.

## 7. 状态、事务与持久化

This design reuses the target Harness session/turn lifecycle and Runtime request/receipt authority; it introduces no new durable Runtime state or status enum.

| Cutover view | Required meaning | Authority |
| --- | --- | --- |
| Session draining | no later turn is routed through the old session after its current turn closes; old identity remains addressable for recovery | Agent Harness |
| Candidate not submitted | advisory candidate tied to old request; no world effect; neither auto-routed to the new provider nor authorized by provider selection | Agent Harness plus existing authorization contract |
| Provider call in flight | old binding remains selected for that request; late response must match old identity; provider timeout does not prove Runtime cancellation | Agent Harness / adapter |
| Runtime pending | same submitted request is unresolved; reconnect queries/replays its identity under the existing Runtime idempotency contract | Runtime |
| Runtime terminal | matching authoritative receipt/disposition closes the old request; only committed receipt proves world effect | Runtime |
| Fresh semantic replan | distinct request lineage, new profile/lane snapshot, independent validation | Agent Harness creates; Runtime judges any submitted candidate |

Provider routing/session evidence may be retained as bounded Agent diagnostics under existing privacy and retention policy. It must not replace Runtime journal/receipt data, and raw prompts, credentials and private provider transcripts are not needed to recover request identity.

## 8. 安全与运行约束

- Provider output and cached response are untrusted decision data. Reconnection must verify the exact session, turn, request, digest and protocol identity before accepting it.
- Missing/ambiguous provider or lane binding, unknown version, digest mismatch or an unresolved old response fails closed; no silent default provider, lane coercion or automatic cross-provider replay.
- A transport-level abort is not a semantic cancel. Only a supported Agent or Runtime cancellation path can produce that result, and the relevant authority must confirm it.
- Existing request timeout, model-call budget, provider credentials and privacy bounds continue to apply. Provider switching does not reset or grant additional calls, budget, action authority or Runtime quota.
- A stalled or unavailable old provider may delay new cognition while its turn is being reconciled. The recovery action must be visible and bounded by the existing timeout/recovery contract; switching providers alone is not a recovery proof.

## 9. 质量与容量

| 场景 | 环境、规模与资源 | 预期响应 | 指标/阈值来源 | 验证入口 | 当前范围 |
| --- | --- | --- | --- | --- | --- |
| Provider/lane cutover during one provider call | one Agent, one active session, one pending decision, fixed observation and profile revisions | old call retains old identity; no new-provider call for that turn; next session uses new binding | zero cross-route dispatch for old request; identity and digest match; timeout remains within existing profile contract | `DES-PSW-004` scenarios PSW-01 and PSW-02 | target design; implementation unverified |
| Cutover after candidate and before submit | same Agent and Runtime head; candidate not yet admitted by Runtime | no automatic handoff/retry; original authorization path or explicit fresh replan only | zero automatic new-provider requests and zero unauthorized Runtime submission | `DES-PSW-004` scenario PSW-03 | target design; implementation unverified |
| Runtime acceptance with lost acknowledgement | same Runtime branch and request identity; reconnect/restart at each acknowledgement boundary | restore original pending/terminal disposition; no second effect | exactly one authoritative outcome for the same identity; conflicting digest fails closed | `DES-PSW-004` scenarios PSW-04 and PSW-05 | paired Runtime proof required |
| Original provider unavailable after switch | bounded timeout, provider reports unavailable or cannot retrieve response | no cross-provider replay; explicit old-turn failure/recovery before fresh replan | no result inferred from transport close; new turn has new identity and causal link | `DES-PSW-004` scenario PSW-06 | target design; provider behavior unverified |

This design sets no new latency or cost threshold. Existing parity policy and provider profile own those limits. Local fixtures can prove identity/dispatch logic only; full-tier parity requires the same fixture/profile with real provider evidence and Runtime receipt/journal recovery.

## 10. 兼容、迁移与回滚

Legacy request contexts that cannot prove the complete provider/lane binding may continue only in their explicitly declared compatibility scope. They cannot be reinterpreted as target parity evidence, upgraded in place, or resumed through a different provider. Existing saved sessions retain the binding/context version with which they started.

Enablement requires a compatible implementation that freezes and verifies the binding, preserves old request identity during reconnect, and passes the negative cutover cases below. To revert provider selection, open a fresh session with the prior profile/lane revision after old turns have settled. Rollback cannot retract a Runtime-committed effect, rewrite a receipt, or make a pending old request terminal; unresolved old requests follow Runtime recovery or operator intervention under its runbook.

## 11. 验证设计与可追溯性

验证应覆盖 Agent route selection、provider response matching、Runtime submission and recovery, and player-visible state projection as separate proof surfaces. Existing unit fixtures are necessary but cannot prove real provider parity or hosted readiness.

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立义务与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | [DES-PSW-001](provider-switch-intent-continuity.design.md#des-psw-001) | session route is immutable; changed provider/lane begins a new bound semantic turn, and lane identity cannot be an unhashed side channel. | Required-tier identity fixtures for PSW-01/02; run the targeted identity suite against one Agent, fixed observation and explicit old/new profile-lane revisions. Source: [agent cognition identity tests](../../../crates/oasis7/src/simulator/tests/agent_cognition_identity.rs). | GitHub Task UID evidence: candidate/source tree, exact command and output, plus code review of digest inputs. | No actual provider support, cost, parity or release conclusion. |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | [DES-PSW-002](provider-switch-intent-continuity.design.md#des-psw-002) | a pre-switch candidate is never passed to the new provider as a retry; any Runtime submission uses the original permitted lineage and current validation. | Required-tier negative fixtures for PSW-03 at pre-submit cutover, with the same Agent, authority and Runtime head. Source: [live Harness tests](../../../crates/oasis7/src/simulator/tests/agent_cognition_live_harness.rs). | Task evidence records the candidate state, provider-call ledger and Runtime result for the same tested tree. | Does not prove Viewer status readability or a real provider's behavior. |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | [DES-PSW-003](provider-switch-intent-continuity.design.md#des-psw-003) | reconnect preserves the submitted Runtime identity/digest and recovers pending/terminal status without a second effect; conflicting digest fails closed. | Required-tier recovery fixtures for PSW-04/05 across lost acknowledgement and restart. Source: [Runtime cognition recovery tests](../../../crates/oasis7/src/runtime/tests/agent_cognition_recovery.rs). | Task evidence contains paired Agent correlation and Runtime journal/receipt readback. | Local recovery fixtures do not prove production storage, multi-node behavior or every provider adapter. |
| [AC-AGENT-PARITY-002](../../product/agents-world-simulation/provider-agent-experience-continuity.prd.md#ac-agent-parity-002) | [DES-PSW-004](provider-switch-intent-continuity.design.md#des-psw-004) | full-tier covers real provider switch/reconnect on the same fixture/profile plus Runtime receipt/journal outcome; no sample widens supported provider combinations. | Full-tier PSW-01..06 on one declared provider pair, lane, fixture and profile; combine real paired provider run with restart/reconnect Runtime evidence. Source: [live pilot parity tests](../../../crates/oasis7/src/simulator/tests/agent_cognition_live_pilot_parity.rs). | Task evidence records provider/profile/protocol, source/integration/tested trees, artifacts and Runtime receipt/journal identities. | A local test alone is not real provider, hosted, release or default-readiness evidence. |

### 11.2 必需场景

- PSW-01: switch provider while the old adapter request is in flight; a late old response matches only the old request, and no request is issued through the new route for that turn.
- PSW-02: change lane with all other inputs fixed; the new lane binding is in the new request digest or a new context version is required; old request digest is unchanged.
- PSW-03: switch after an Agent candidate exists but before Runtime submission; do not invoke the new provider with the old candidate or submit it without its prior authorization path.
- PSW-04: Runtime accepted the exact submitted request but the acknowledgement/receipt was lost; reconnect recovers the original identity and one receipt/effect.
- PSW-05: retry the same Runtime identity with a different digest; fail closed and produce no second effect.
- PSW-06: old provider becomes unavailable after cutover; do not replay its old request through the new provider; after old-turn recovery, a fresh turn carries new route identity and causal lineage.

## 12. 决策、长期风险与未决问题

| 决策 | 选择与理由 | 代价 / 风险 | Owner / 复核触发 |
| --- | --- | --- | --- |
| Provider/lane change starts a new immutable session binding | Fits existing session profile revision and prevents late response rebinding. | An unresolved old turn may delay next cognition. | Agent owner; review if Harness replaces session-scoped profile revision. |
| Runtime identity is reused only for recovery of the same submitted request | Preserves Runtime idempotency and authoritative result; fresh Agent intent gets fresh lineage. | Requires reconnect to locate old correlation and receipt. | Runtime owner; review on request/receipt contract change. |
| No automatic cross-provider fallback | Avoids changing the model decision behind an existing identity or replaying old candidate. | Provider outage can reduce continuity until an explicit new turn is safe. | Product/Agent owners; review only with an explicit fallback contract and paired QA evidence. |

Residual risk: current target documents establish stable provider profile, request digest, single-flight, retry, Runtime idempotency and receipt recovery separately, but do not prove that each live route includes lane selection in its digest or that all adapters can recover the same response after process restart. `AgentIntentV2` chat-path identity is scoped to its own actor/authority/message inputs and is not proof that provider/lane identity is bound for every parity lane. The implementation owner must verify the actual route and digest; otherwise the affected route remains outside this contract.

No unresolved schema, support-matrix or player-promise decision is delegated by this design. Any change to action semantics, Runtime status mapping, player messaging or supported provider combinations returns to the corresponding owner through TPM.

<a id="des-psw-001"></a>
### DES-PSW-001：session provider/lane binding is immutable

Each Agent session resolves its provider configuration reference, profile revision, adapter protocol version and execution-lane selection/revision before its first turn. The effective binding is immutable for all requests in that session. Every target request digest must bind that complete selection, directly or through a versioned profile/context reference that deterministically resolves it. An unbound lane is not target evidence.

A provider/lane change drains the old session and applies to a new session/turn only. It does not rewrite any old context or digest. The runtime meaning and permission scope of a lane do not change here.

<a id="des-psw-002"></a>
### DES-PSW-002：pre-switch candidates keep their original lineage

A candidate is advisory output belonging to the request that produced it. Provider cutover never passes that candidate payload to the newly selected provider as a retry and does not grant permission to submit it. If the old candidate follows its existing authorization path, it retains its original request lineage and still undergoes current Runtime validation. When a fresh provider decision is needed, Agent creates a new semantic request with a causal reference to the old turn; the new provider receives a new observation/context and cannot inherit old request identity.

<a id="des-psw-003"></a>
### DES-PSW-003：reconnect recovers one authority chain

Reconnect resolves the original provider binding for an unresolved provider call. Transport retry keeps the same request identity and digest and is allowed only under same-key idempotency or response-retrieval semantics. A provider that cannot provide that guarantee cannot be replaced in-place; the old turn uses its existing failure/recovery path.

Once submitted, a request stays on its original Runtime authority chain. Lost acknowledgements or receipts are recovered by querying/retrying that exact Runtime identity/digest. Pending remains pending until Runtime returns an authoritative terminal disposition. Same identity plus different digest fails closed. Cutover and reconnect never infer cancel, reject, commit, expiry or a second world effect.

<a id="des-psw-004"></a>
### DES-PSW-004：cutover evaluation separates required and full evidence

Required-tier fixtures cover PSW-01 through PSW-06 negative and identity cases, including late old responses, unsubmitted candidate handoff, lost acknowledgement, pending status and conflicting digest. Full-tier evaluation runs the declared real provider pair on the same fixture/profile and lane, then pairs provider trace with Runtime receipt/journal evidence across restart/reconnect. Candidate selection rules and provider scope come from the linked parity authority; no single mock, local fixture, historical sample or design check expands provider support, default eligibility or release readiness.
