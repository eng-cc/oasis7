# LLM Agent Decision Provider 标准层 + Local Provider 外部适配可行性（2026-03-12）设计

- 对应需求文档: `doc/world-simulator/llm/decision-provider-contract.prd.md`
- 专题入口与权威边界: `doc/world-simulator/llm/README.md`

## 1. 设计定位
定义 world-simulator 中“世界内 Agent 契约”与“具体外部 agent provider”之间的标准层，使 `Local Provider` 之类的外部框架可通过 adapter 参与模拟，但不侵入 runtime 权威、规则执行与回放边界。

### 1.1 实现状态与证明门槛

本设计将接口存在、显式 bridge 和生产闭环分开记录：

| 状态 | 结论 |
| --- | --- |
| `current` | `DecisionProvider`、Mock/loopback contract、active manifest 的确定性 command projection、`CapabilityCatalogSnapshot`/`CapabilityInvocationContext`/`AgentCommandResponse` DTO 及基础校验已存在。 |
| `partial` | `ProviderBackedAgentBehavior` 可通过显式 context 接收 catalog，loopback 可校验 typed response，且有 `RuntimeTrustedModuleExecutor` seam；尚无证据证明每个 live Agent turn 由 runtime 自动生成并注入 subject-bound context。 |
| `partial` | 当前 catalog 主要是身份、schema version/hash 和 payload bound；`payload: Vec<u8>` 是 opaque bytes，envelope canonical CBOR 校验不等于制度参数 schema 的语义校验。 |
| `target` | runtime-owned per-turn discovery、受信 semantic schema retrieval、typed args 的 host canonical encoding、runtime 二次校验和未知制度 E2E。 |
| `proven` | 只有固定 fixture 同时证明成功闭环与负例无 sandbox/journal/state/charge 副作用，才能把该 target 标记为 proven。 |

因此，当前文档中的“catalog 已落地”仅指机器可发现的 projection/DTO seam；“Agent
capability discovery 已落地”必须等自动 wiring 与真实 Agent E2E 通过后才可使用。

## 2. 设计结构
- 世界权威层：`WorldKernel / runtime` 继续负责校验、状态变更、事件与 receipt。
- Agent 外观层：保留 `AgentBehavior` 作为 simulator 现有调用入口，对上保持 `decide/on_event/on_action_result` 语义。
- Decision Provider 标准层：在本地定义稳定的 request / response / feedback / trace 契约，作为唯一 provider 集成点。
- Provider Adapter 层：为 `Local Provider`、本地 mock provider、未来其他 agent framework 分别实现适配器。
- Memory / Trace 桥接层：把 provider transcript、tool trace、diagnostics、memory write intent 收敛到本地 `AgentMemory` 与 `AgentDecisionTrace`。
- 验证与评估层：通过 golden fixtures、错误签名与 QoS 指标，对不同 provider 进行横向评估。

### 2.1 目标 discovery / execution sequence

```text
WorldKernel live state
  -> active manifest + subject grant/policy intersection
  -> CapabilityCatalogSnapshot + CapabilityInvocationContext (one turn)
  -> schema_ref retrieval and canonical schema_hash check
  -> DecisionRequest(observation, catalog, context, semantic schemas)
  -> provider returns TypedCommandIntent
  -> trusted host validates typed args and encodes canonical CBOR envelope
  -> runtime revalidates catalog/context/schema/authz/cost/nonce/live state
  -> staged module execution -> committed ActionResult/receipt
  -> FeedbackEnvelope -> local memory policy
```

Runtime must own the first, second and final validation steps. The provider may choose only
an entry already present in the snapshot and may propose typed values; it cannot select the
subject, presenter, audience, grant, snapshot, nonce, caller, or cost quote. The context is
transport binding, not a bearer grant; grant, revocation, policy, budget and replay checks
remain runtime authority.

The per-turn generator is a target host service (not a provider feature):

1. Read the finalized live head, active module records, policy and the Agent subject's
   non-revoked grants for the requested audience.
   The immutable `AgentInvocationBinding` digest binds `world_id`, `branch_id`,
   `finality_epoch`, `finality_block_hash`, `policy_epoch`, `revocation_epoch`,
   catalog snapshot digest, exact audience/instance target and response nonce.
   These fields are part of the target canonical hash domain; omission from a
   compatibility DTO cannot silently omit the binding and instead rejects target execution.
2. Intersect those records with declared commands, sort entries deterministically, compute
   the snapshot identity and choose a fresh response nonce. Store/bind the invocation context
   before making the provider request.
3. Resolve each eligible `schema_ref` from a trusted manifest/schema store. Expose only a
   verified descriptor to the provider and keep the grant id/eligible grant list out of the
   provider's authority-bearing response.
4. Invalidate the snapshot at its tick/head/policy/revocation boundary; a changed catalog
   requires a new discovery request, not an automatic rebind or fallback.

The current explicit `with_capability_context` constructor path is useful for fixtures and
host-loop experiments, but it is not the per-turn generator and must not be described as
production auto-wiring.

## 3. 关键接口 / 入口
- `AgentBehavior`：`crates/oasis7/src/simulator/agent.rs`
- `AgentDecisionTrace`：`crates/oasis7/src/simulator/agent.rs`
- `AgentMemory`：`crates/oasis7/src/simulator/memory.rs`
- viewer 诊断协议：`crates/oasis7_proto/src/viewer.rs`
- runtime-live bridge 消费 `AgentDecisionTrace`；本专题拥有 request/response/feedback/trace/memory 语义，Viewer delivery 与 runtime event/snapshot mapping 分别由 `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md` 与 `doc/world-simulator/prd.md` 承载。
- launcher operator HTTP-JSON 机器接口专题：`doc/world-simulator/launcher/game-client-launcher-control-plane-and-machine-interface.prd.md`；该接口不等同于 `DecisionProvider` 或世界内 autonomous Agent。

## 4. 约束与边界
- 外部 provider 只提供决策建议，不得直接改 world state、存储或 runtime 内核。
- 所有 world action 必须先经过本地 action schema 白名单，再进入 runtime 校验。
- provider trace 需要可脱敏、可裁剪、可映射；不得把外部协议泄露为 viewer 唯一调试口径。
- trace 必须有界并脱敏，同时保留 provider latency、token/cost、repair diagnostics 与 action/result 关联。
- memory 的权威副本保留在本地 world-simulator；外部 memory 仅可作为 provider 内部缓存或检索辅助。
- `Local Provider` PoC 只允许在低频、低破坏性 agent 类型上试点；高频强一致 actor 不在首轮范围。
- required 测试必须可离线执行，因此标准层必须先支持 `MockProvider`，不能以外部联网能力作为主验证前提。

### 4.1 Semantic schema 与 typed args

`ModuleCommandCatalogEntry` 是 discovery identity，不是足够的 Agent 语义。target 为每个
entry 绑定 `schema_ref` 和 content-addressed `schema_hash`，descriptor 的 canonical
内容至少包含：command summary、input fields（name/type/required/default/bounds/enums）、
result/error shape、声明的可见副作用与 bounded cost hint。descriptor 不能包含私钥、
grant proof 或 host-only provenance，也不能授予 command 权限。

schema retrieval 由 trusted host 完成：

```text
active manifest declaration
  -> schema_ref lookup
  -> canonical descriptor bytes
  -> hash/version/field validation
  -> provider semantic tool schema
```

找不到 descriptor、canonical bytes 无法解码、版本不支持或 hash 不匹配时，host 在 provider
调用前返回稳定的 `capability_schema_unavailable` / `capability_schema_mismatch`。provider
不得用自己的 schema、自然语言相似度或历史缓存补洞。

target provider response 是 `TypedCommandIntent`：

```text
{
  catalog_snapshot_id,
  response_nonce,
  selected_entry: { module_id, module_version, instance_target,
                    namespace, command, schema_version, schema_hash },
  typed_args,
  trace_id
}
```

For a module-instance audience, `instance_target` is mandatory and carries the stable
`(world_id, module_id, instance_id)` key through snapshot, response, invocation digest,
nonce and execution receipt. A governed world/institution-scoped command uses an explicit
non-instance target variant; missing target is never interpreted as global-module fallback.

trusted host 依据已验证 descriptor 检查 typed args 的必填字段、类型、范围、枚举和大小，
再以唯一 deterministic canonical-CBOR 规则编码为 `ModuleCommandEnvelope.payload`，并
由 host 填充 namespace/name/schema version/hash。provider supplied raw bytes 只能作为
transitional compatibility lane，不能绕过 host encoding；当前 `decode_canonical` 只验证
envelope bytes 的 canonical form，不代表 opaque payload 已按制度 schema 通过语义验证。

runtime 随后重新执行 envelope canonical encoding、payload 完整语义解码、active module
和 declaration/hash/limit、subject/grant/policy/revocation、cost quote、nonce/replay
和 live head 检查。任一检查失败都不得进入 sandbox、metering、journal 或 partial state。

## 5. 设计演进计划
- 先冻结 `Decision Provider` 最小契约与 fixture 评估口径。
- 再实现 `MockProvider`，验证标准层与本地 trace / memory / action mapping 正常闭环。
- 再实现 `Local ProviderAdapter` PoC，并限制到单一低频 NPC 场景。
- 最后依据动作有效率、超时率、成本与诊断完整度，决定是否扩展到更多 agent 类型。

### 5.1 Proof-first 交付顺序

1. 在 runtime host 增加 per-turn subject-bound catalog/context 生成和注入，保留显式
   constructor seam 作为 fixture-only 路径。
2. 为 active module command 建立 semantic schema retrieval，验证 canonical descriptor
   hash 与 manifest declaration 一致。
3. 让 provider 只返回 typed args，由 host canonical encode，再由 runtime 做完整重验。
4. 用一个不在 Kernel closed `Action` 枚举中的 active governed module command 做未知制度
   E2E；成功后再推进统一 transaction、instance-targeted command/state 等 runtime 扩展。
5. 固定 scenario、agent profile、provider/adapter/protocol version、catalog/context、
   timeout、tick budget 和 cost budget，重复采样并单独报告 schema failure、context
   contamination、drift、stuck-loop、latency/token/cost；聚合均值不能掩盖单场景失败。

### 5.2 未知制度 E2E 与负例

成功 fixture 选择一个未进入 Kernel closed `Action` 的 command（例如
`economic_contract.open`），但该 command 所属 module 已通过治理并 active：

`live head -> subject grant/policy intersection -> catalog/context -> schema retrieval ->
provider typed args -> host canonical CBOR -> runtime live recheck -> staged module execute ->
committed result/receipt -> feedback -> memory policy`。

验收必须证明不添加 native Action、不接受 provider 自报 grant/schema、不依赖手工复制
context，也不把 provider 过程当作 world fact。至少覆盖下列拒绝：

| 负例 | 预期拒绝与副作用保证 |
| --- | --- |
| unknown/inactive module 或 command 不在 snapshot | stable error，收敛为 `Wait`/`ActionRejected`，不进 sandbox/journal/state/charge |
| schema missing、hash/version 漂移或无法 canonical decode | 在 provider 调用前 fail closed |
| revoked/expired grant 或 subject/presenter/audience mismatch | runtime deny，不按名称、旧 grant 或静态 Action fallback |
| stale snapshot、policy/branch/finality/cost quote 变化 | old intent 失效，要求重新 discovery |
| typed args 缺字段、类型/范围/枚举错误或超 bound | host 不生成 envelope |
| nonce replay 或同 nonce 不同 request hash | 返回历史 idempotent receipt 或 deterministic denial，不重复副作用 |
| world/branch/finality、policy/revocation epoch 或 snapshot digest 漂移 | old context 失效并重新 discovery，不跨 authority boundary 重放 |

The successful unknown-Institution fixture reuses runtime
`institution-migration-v1` and product SC-32: preview/stale/deny/no-effect
outcomes debit nothing, one accepted effect has one debit and receipt, and
retry/reconnect/restore/replay neither double-charge nor create replay credit.

## 6. 内置 provider 适配与诊断流

- builtin Responses adapter 将感知、裁剪后的本地 memory、目标、有限工具 catalog 与 timeout budget 组装为 request；它只返回一个结构化候选。profile/provider 覆盖、endpoint 规范化、有限 retry 与错误分类属于 adapter 配置层，不能改变 world state 或 runtime 权威。
- **Target closed-action safety flow** 固定为 `perceive/context/goal -> request -> one structured decision -> catalog/schema guard -> host adapter -> runtime validate/execute -> committed ActionResult feedback -> Agent memory`。当前 builtin `AgentRunner` 只产出 decision，`action_result=None`，没有内建 runtime executor callback；只有显式 loopback/host seam 能继续执行，因此 runner decision 或 parser 成功都不是 committed feedback 证据。无终态、超量 repair、非法动作、超时或 provider failure 收敛为可诊断的 `Wait`/`ActionRejected`；不执行多片段 completion。
- Trace bridge 保存角色分离的 chat/tool trace、预算/latency/repair diagnostics、module-call intent/receipt 与 feedback。它裁剪和脱敏后供诊断使用；Viewer delivery、runtime event/replay 与玩家记忆纠正仍由各自专业 authority 负责。
- receipt flow 是 `prompt module-call -> diagnostic intent/receipt -> journal/trace -> later reconciliation`。它保留上下文与兼容默认值，但并未证明 action-result 因果、重放不重调 provider 或恢复不重复外部副作用；这些 T4/T5 proof 必须由 runtime 设计与验证完成。

### 6.1 验收证据分层

| 证据层 | 能证明什么 | 不能证明什么 |
| --- | --- | --- |
| required | DTO round-trip、catalog/context binding、semantic schema hash、typed args host encoding、稳定错误码和 Mock 负例 | 真实 provider 可理解未知制度，或 live runtime 已自动 wiring |
| full | 未知制度 E2E 的成功/负例、runtime live recheck、staged commit、committed feedback/memory、replay/no-duplicate-side-effect | 更大规模默认体验或产品规则正确性 |
| rollout/parity | 在固定 profile/场景下的行为、时延、成本、trace 完整度和稳定性 | 任何 provider 绕过 runtime authority 的许可 |

只有 full 证据同时覆盖成功链路和上表负例，才可将 subject-bound discovery、semantic
schema、typed args execution 标记为 `proven`；单独的 catalog DTO、schema parser、
provider response validator 或诊断 trace 均只能保持 `partial`。
