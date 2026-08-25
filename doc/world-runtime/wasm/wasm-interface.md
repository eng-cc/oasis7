# oasis7 Runtime：WASM 扩展接口与 ABI（设计分册）

审计轮次: 4

本分册为 `doc/world-runtime/prd.md` 的详细展开。

SDK 的默认 no_std、共享 Canonical-CBOR wire 类型、codec 错误与 builtin 兼容 evidence 由 `doc/world-runtime/wasm/wasm-sdk.prd.md` 承载；本文继续作为 runtime ABI/reference 入口，不承担 SDK 实现任务账本。

`crates/oasis7_wasm_abi` 是 `ModuleManifest`、ABI contract、artifact identity 与 limits 类型的单一代码来源；net/proto/viewer 不得维护重复定义。历史 distributed Phase 7 移除 `oasis7_net` 重复 manifest 属于已完成 crate-boundary provenance，不构成第二套现行规范。

## WASM 扩展接口（草案）

> 目标：允许 Agent 自行设计“新事物”模块（Rust → WASM），由世界内核以事件/接口动态调用；模块只产生确定性计算与显式 Effect 意图。

**ModuleManifest（控制面条目）**
```rust
struct ModuleManifest {
    module_id: String,         // 内容地址或哈希
    name: String,
    version: String,           // 语义化版本
    kind: ModuleKind,          // Reducer / Pure（后续可扩展）
    role: ModuleRole,          // Rule / Domain / Gameplay / Body / AgentInternal
    wasm_hash: String,         // 模块工件哈希
    interface_version: String, // 例如 "wasm-1"
    exports: Vec<String>,      // 导出函数名
    subscriptions: Vec<ModuleSubscription>,
    required_caps: Vec<CapabilityRef>,
    limits: ModuleLimits,      // 沙箱资源上限
}
```

**ModuleKind**
- `Reducer`：有状态的确定性 reducer（输入事件 → 新状态 + Effect 意图）
- `Pure`：无状态纯函数组件（输入 → 输出）

**ModuleRole**
- `Rule`：动作校验/成本评估等规则模块
- `Domain`：经济、社会、治理等领域规则
- `Gameplay`：战争/危机/经济玩法协议与状态机模块
- `Body`：机体/零件/耐久等物理外层逻辑
- `AgentInternal`：记忆/工具/规划等内部能力

**Agent 内部模块（记忆/工具）**
- 仍使用 `ModuleManifest`/`ModuleLimits` 与统一 ABI；由 Agent runtime 触发调用，不走 event/action 订阅路由。
- 记忆模块通常使用 `Reducer`，以 `state` 作为受限持久存储；运行时对状态大小/条目数施加配额（实现可为专用存储或状态分片）。
- 模块保持确定性计算，不直接调用外部 I/O；LLM/推理服务调用在模块外部完成。

**ModuleSubscription**
- `event_kinds`: Vec<String>（订阅的事件类型）
- `action_kinds`: Vec<String>（可选，订阅的动作类型）
- `stage`: `"pre_action" | "post_action" | "post_event" | "tick"`（动作/事件/周期路由阶段）
- `filters`: 可选过滤条件（例如仅关注某类 owner/地点）

**Tick 生命周期调度（执行器合同）**
- 只有激活且订阅 `tick` 的模块实例进入调度表；激活时初次安排在当前 logical tick。调度表以 `instance_id -> next_wake_tick` 持久化在世界 snapshot 中，因此 restart/replay 不得把尚未到期的唤醒丢失或提前。
- `step_with_modules` 每次推进 logical time 后只调用到期实例。tick 调用的 `ctx.stage` 为 `"tick"`，`event` 与 `action` 均省略；reducer 仍携带其受限 `state`，pure 模块不携带 `state`。
- 执行器在调用前移除该实例的旧 schedule。调用成功后，只有 `tick_lifecycle.wake_after_ticks` 才写入下一次 schedule；`ticks=0` 按 `1` clamp。`tick_lifecycle.suspend` 或字段缺省均不重排。调用失败同样不会自动恢复已消费的 schedule，宿主必须返回结构化失败而不能静默重试。
- 这一定时语义不改变模块权限、artifact identity 或 manifest hash 规则；新增/变更 tick subscription 仍按普通 manifest 和 artifact 校验链处理。

**ModuleLimits（示意字段）**
```rust
struct ModuleLimits {
    max_mem_bytes: u64,        // 线性内存上限
    max_gas: u64,              // 指令燃料
    max_call_rate: u32,        // 每 tick 最大调用次数
    max_output_bytes: u64,     // 输出上限（ModuleOutput 编码后大小）
    max_effects: u32,          // 单次调用最大 effect 数量
    max_emits: u32,            // 单次调用最大 event 数量
}
```

**Reducer 调用签名（示意）**
```rust
fn reduce(event: WorldEvent, state: Bytes, ctx: ModuleContext) -> ModuleOutput
```

**Pure 调用签名（示意）**
```rust
fn call(input: Bytes, ctx: ModuleContext) -> Bytes
```

**ModuleContext / ModuleOutput（示意）**
- `ModuleContext`：`{ time, origin, stage?, world_config, module_id, trace_id }`
- `ModuleOutput`：`{ new_state, effects: Vec<EffectIntent>, emits: Vec<WorldEvent> }`

**模块生命周期事件（占位）**
- `RegisterModule / ActivateModule / DeactivateModule / UpgradeModule`
- 以事件写入日志，支持审计与回放

**模块失败事件（当前）**
- `ModuleCallFailed { module_id, trace_id, reason }`
- 加载/校验阶段失败当前映射到 `ModuleCallFailed`（由 `code/detail` 区分）。

**规则决策事件（占位）**
- `RuleDecisionRecorded { action_id, module_id, stage, verdict, override_action?, cost, notes }`
- `ActionOverridden { action_id, original_action, override_action }`
- Rule 模块通过 `ModuleOutput.emits` 输出 `kind="rule.decision"`，`payload` 为 `RuleDecision` 的 JSON 序列化。
- simulator 的 historical `WorldKernel` adapter 可将单个 pre-action call 映射到这一 payload；它不扩展本 ABI 的 module lifecycle、artifact governance 或多模块编排语义，且无效 payload / action id 必须由 adapter 结构化拒绝，不能静默降级。

## ABI 与序列化（草案）

> 目标：模块与宿主之间的输入/输出采用**确定性**编码，保证回放与跨平台一致性。

**编码格式**
- 使用 **Canonical CBOR**（键排序、确定性编码）。
- 禁止 NaN；浮点仅在明确字段允许时使用（默认使用整数与字节串）。
- `GeoPos` 与所有 `*_cm` 坐标/尺寸字段在 action/event/observation 边界上只接受并输出 CBOR/JSON 整数厘米值，不接受 float 类型；仅模块持久化状态解码可兼容读取历史上的整值浮点厘米表示。
- `Bytes` 一律使用 CBOR byte string。
- WasmExecutor 输出解码使用 Canonical CBOR（失败映射为 InvalidOutput）。

**ModuleContext（CBOR Map）**
```
{
  "v": "wasm-1",
  "module_id": "...",
  "trace_id": "...",
  "time": i64,
  "origin": { "kind": "event|action|system", "id": "..." },
  "stage": "pre_action|post_action|post_event|tick", // action/event/tick 路由阶段（可选）
  "world_config_hash": "...", // 当前 manifest 哈希
  "limits": { "max_mem_bytes": u64, "max_gas": u64, "max_output_bytes": u64 }
}
```

**Reducer 输入（CBOR Map）**
```
{
  "ctx": ModuleContext,
  "event": Bytes,   // WorldEvent 的 canonical CBOR
  "state": Bytes    // reducer 当前状态（canonical CBOR，若无则为空字节串）
}
```

**Pure 输入（CBOR Map）**
```
{ "ctx": ModuleContext, "input": Bytes }
```

**ModuleCallInput（当前运行时）**
- 当前 runtime 以 `ModuleCallInput { ctx, event?, action?, state? }` 作为调用输入封装。
- `event`/`action` 字段均为 canonical CBOR bytes。
- reducer 调用会携带 `state`（空字节串代表无历史状态），pure 调用省略 `state`。
- `pre_action` 阶段仅提供 `action`；`post_action` 阶段提供 `action` + `event`（动作落盘后的结果事件）；`post_event` 阶段仅提供 `event`。
- `tick` 阶段不提供 `event` 或 `action`；其 schedule/重排语义以上述执行器合同为准。

**ModuleOutput（CBOR Map）**
```
{
  "new_state": Bytes | null,
  "effects": [ Bytes ], // EffectIntent 的 canonical CBOR 列表
  "emits": [ Bytes ],   // WorldEvent 的 canonical CBOR 列表
  "tick_lifecycle": { "mode": "wake_after_ticks", "ticks": u64 }
                    | { "mode": "suspend" } // 可选；缺省表示不重排
}
```

- 当 `new_state` 为非空时，运行时记录 `ModuleStateUpdated` 事件并更新模块状态。
- Pure 模块不得返回 `new_state`，否则视为 InvalidOutput。
- `tick_lifecycle` 是 ABI 的可选、serde-defaulted 扩展：旧模块输出省略该字段仍可解码，且其既有非-tick 调用行为不变；若该输出来自 tick 调用，缺省的兼容含义是 suspend（不再调度），而不是隐式每 tick 重试。
- 回归至少覆盖：激活初排、仅到期调用、`wake_after_ticks` 重排、`suspend`/缺省不重排、零 ticks clamp、调用失败不重排，以及 schedule 的 snapshot round-trip。

**错误约定**
- 模块返回非规范 CBOR、输出超限或字段缺失时，宿主记录 `ModuleCallFailed` 事件并拒绝输出。

## 开放制度扩展合同（试点，非全面迁移）

本节把“制度能力由 WASM 扩展”的合理部分落成一个可审计的增量合同。它是
现有 `wasm-1` ABI 的可选扩展，不是立即把 `WorldState` 改成 ECS、删除既有
native action，或允许任意未治理工件进入生产执行路径。

试点只选择一个有界制度（例如 Alliance 或 EconomicContract，由跨角色治理
确定），以现有 native vertical slice 作为兼容基线；试点成功前不得把所有
经济、战争或治理能力迁出内核。模块治理、工件发布和实例生命周期继续遵循
`doc/world-runtime/module/module-lifecycle.md` 的 `propose -> shadow ->
approve -> apply` 闭环。

### 1. 命名空间、声明和版本

模块的开放对象必须在 manifest 中声明。声明是可选的、serde-defaulted ABI
扩展；缺省时等同于“没有开放制度对象”，历史 manifest 和已有 `wasm-1`
模块无需修改即可读取。

```rust
struct ModuleSchemaDeclarations {
    commands: Vec<CommandSchema>,
    components: Vec<ComponentSchema>,
    events: Vec<EventSchema>,
    migrations: Vec<SchemaMigration>,
}

struct SchemaRef {
    name: String,
    schema_version: u32,
    schema_hash: String, // SHA-256 of the canonical schema descriptor
    max_payload_bytes: u64,
}

struct SchemaMigration {
    object_kind: String, // "component" or "event"
    namespace: String,
    name: String,
    from_schema_version: u32,
    to_schema_version: u32,
    from_schema_hash: String,
    to_schema_hash: String,
    migration_wasm_hash: String,
}
```

`ModuleSchemaDeclarations` 由当前 `ModuleManifest.abi_contract` 的可选扩展
承载；它不能替代 `interface_version`、`input_schema`、`output_schema`、
`required_caps` 或 `cap_slots`。ABI 版本、对象 schema 版本、manifest hash
和工件 `wasm_hash` 是四个不同的身份维度：任一声明、权限、limits 或
metering schedule 改变，都必须产生新的 manifest hash；工件字节变化必须
产生新的 `wasm_hash`，不得同名覆盖。

命名约束如下：

- `namespace` 是由治理绑定到 module identity 的小写、稳定标识（示例
  `bank.acme`）；`command`、`component`、`event` 是 namespace 内的本地名，
  三者的完整键分别为 `namespace:command`、`namespace:component`、
  `namespace:event`。
- `core.*`、`kernel.*` 和内核保留对象不可由模块注册或覆盖。命名空间不能
  通过上传新工件重新归属；归属变更必须走新的治理提案和 manifest hash。
- 同一完整键的 schema 版本必须显式递增。旧版本仍可用于历史事件和回放；
  未知的未来版本在边界处 fail closed，不能按“最接近字段”猜测解码。
- schema descriptor、声明列表、capability 绑定和 limits 在计算 manifest
  hash 前按完整键和版本排序；模块执行时的访问集合也必须按同一稳定顺序
  处理，不能依赖 map/hash iteration 顺序。

### 2. Command / component / event wire

开放制度命令不为每一种银行、联盟或合约操作新增一个闭合的 Rust `Action`
变体。受治理的 module-call/router 入口承载如下 Canonical-CBOR command
envelope；现有 `ModuleCallInput.action` 仍是兼容承载槽位。

```text
{
  "namespace": "bank.acme",
  "command": "open_account",
  "schema_version": 1,
  "payload": <canonical-cbor bytes>
}
```

宿主从 `ModuleContext` 注入 caller、origin、logical time、trace/journal
位置；模块不得在 payload 中伪造这些字段。宿主必须先确认模块处于 active
状态、完整命中 manifest 声明、`payload` 哈希/大小和 schema 版本有效，再进
入 WASM；未知 namespace、command 或版本均结构化拒绝。

模块状态采用逻辑 component 记录，但这不是本期承诺的完整 ECS 或
`WorldState` 重写：

```text
ComponentKey   = { namespace, component, schema_version, entity_id }
ComponentValue = { schema_version, data: canonical-cbor bytes }
```

模块只能读写自己声明且被授予 capability 的 namespace；模块局部 singleton
使用稳定的 instance/entity key。核心位置、身份、所有权、时间、资源守恒和
其他 Kernel invariant 不作为可写 component 暴露。需要改变核心状态时，模
块只能产生经过 capability/policy 校验的 `EffectIntent`，由 Kernel 原子应用
或拒绝。

模块发出的事件必须匹配 manifest 的 event declaration，且完整键为
`namespace:event`；宿主负责填充 event id、logical time、caused-by 和当前
manifest hash。payload 按声明的 schema 以 Canonical CBOR 校验；现有
`ModuleEmit`/legacy event 表示可作为兼容 adapter，但不得因为 legacy JSON
形状而跳过 schema/hash 校验。事件的版本和原始 payload 保留在事件日志中，
不得原地重写旧事件。

### 3. Capability、权限和 Kernel 边界

- command、component read/write、event emit 都必须在 manifest 中声明所需
  `cap_ref`；`cap_slot` 只能解析到 manifest 中唯一且已授予的 capability。
  模块不能自授予 capability、扩大 entity 范围或把 read 提升为 write。
- capability 是最小作用域的治理授予（namespace、对象类型、实体范围和
  expiry 可选）；缺失、过期、冲突或 policy deny 在执行前返回结构化
  `CapsDenied`/`PolicyDenied`，不应用 state、effect、emit，也不产生部分扣费。
- 默认 host 不暴露真实时间、随机数、网络、文件或任意 I/O。所有外部影响
  必须表达成带 `cap_ref` 的 `EffectIntent`，由宿主 policy 决定是否落账。
- 模块永远不能覆盖 Kernel invariant、绕过资源守恒、跳过 logical time、
  直接移动/写入核心 component，或把制度权限转化为共识/节点权限。制度语义
  仍由其领域治理和试点分册拥有，本合同只定义可验证的扩展边界。

### 4. 确定性计量和执行收据

执行必须同时受 WASM fuel/`ModuleLimits.max_gas` 和资源计量 schedule 约束。
当前基线 `wasm-runtime-v1` 沿用 executor 权威中的确定性费用口径：

```text
compute = ceil(input_bytes / 1024)
        + ceil(output_bytes / 1024)
        + 2 * effect_count
        + emit_count
electricity = 1 + effect_count + emit_count + has_new_state
```

若试点增加 component read/write 或 schema decode 费用，必须使用新的、明确
命名的 `metering_schedule_version`；费用只能由 canonical 字节长度、声明的
访问数量、WASM fuel、effect/emit 数等确定性输入计算，不得使用 wall-clock
耗时或未排序的集合。执行收据至少绑定 module/instance、manifest hash、
`wasm_hash`、schedule version、输入/输出大小、fuel/费用、effect/emit 数、
状态前后 hash 和 journal position。

余额或 fuel 不足必须在提交前返回结构化拒绝；成功调用才产生计费和状态/事件
变更。回放消费已记录的执行结果与计量收据，不重新按当前价格或当前 schedule
计算历史费用；新 schedule 只影响新 manifest/新执行。

### 5. 安装、升级、禁用和迁移

开放对象的 install/register、activate、upgrade、deactivate 必须与 manifest、
artifact identity 和 `ModuleChangeSet` 同一治理闭环绑定：

- install 只接受已计算并重新校验的 SHA-256 `wasm_hash`，以及发布链提供的
  artifact identity/build receipt；本地任意源码编译或上传不会自动激活模块。
- upgrade 必须引用当前 `base_manifest_hash` 和 active version。保持同一
  ABI/schema 的兼容升级可复用已有 state；若 component/event schema hash
  改变，manifest 必须声明对应对象类型的确定性 `SchemaMigration`，否则拒绝
  升级。也可以注册一个新的完整 event key，让旧 event 保持原 schema。
- disable 在逻辑 tick 边界停止新调用，但保留 module state、旧 manifest、
  artifact hash 和完整事件历史；禁用不是删除，也不能让回放选择另一版本。
- apply 失败是原子的：不写半套 register/activate/upgrade/deactivate 事件，
  不更新 active registry，不覆盖旧 artifact/state。事件序列和同类排序继续
  服从 `module-lifecycle.md` 的现有合同。

迁移声明至少绑定对象类型、`namespace/name`、from/to schema version/hash、
迁移工件 `wasm_hash` 和迁移 manifest hash。component 迁移函数必须是确定性、
受限、无外部 I/O 的纯 state 转换，只能产生目标 namespace 的 state；event
迁移只能把历史 payload 转成已声明的 reducer 输入，不能伪造 caller、time、
caused-by 或 Kernel event。宿主在提交 component 迁移时记录：

```text
ModuleStateMigrated {
  instance_id, entity_id, namespace, component,
  from_schema_version, to_schema_version,
  from_state_hash, to_state_hash,
  migration_wasm_hash, manifest_hash, caused_by,
}
```

回放先按旧 schema 重放历史事件，到迁移边界应用 state/event 的声明迁移，再
按新 schema 继续；历史 event/state bytes 不重写。没有 event migration 时，
旧 event 必须按其原版本交给仍声明支持它的 reducer，不能静默按新 schema
重新解释。迁移失败、hash 不匹配或输出超限时保留旧版本并结构化拒绝。
manifest 自身的版本迁移仍沿用
`module-lifecycle.md` 的 `ManifestMigrated` 语义，并要求 from/to hash 可追溯。

### 6. 试点验收与非目标

一个 Alliance 或 EconomicContract 试点在进入实现前，至少需要证明：

1. 一个 namespace 能声明 command/component/event schema，并以 canonical
   envelope 被发现、校验、调用和拒绝未知版本。
2. capability 不足、Kernel component 写入、fuel/limits 超限、余额不足和
   artifact/manifest hash 不一致均 fail closed，且无部分 state/effect/event。
3. register/activate/upgrade/deactivate、state migration、snapshot/replay
   在同一 journal 上得到一致 state/event/hash/计量收据。
4. 既有 native vertical slice、旧 `wasm-1` manifest 和未参与试点的模块行为
   保持兼容。

本试点不承诺全面 ECS、删除 `tick_consensus`、把所有制度迁入 WASM、动态
  world database、分片，或接受任意未治理上传；这些仍需独立的 runtime、
  consensus、产品和安全评审。

## 关键数据结构（草案）
- `WorldEvent`：`{ id, time, kind, payload, caused_by }`
- `EffectIntent`：`{ intent_id, kind, params, cap_ref, origin }`
- `EffectReceipt`：`{ intent_id, status, payload, cost?, timestamps, hash }`
- `CapabilityGrant`：`{ name, cap_type, params, expiry? }`
- `PolicyRule`：`{ when, decision }`
- `Manifest`：`{ reducers, modules, module_changes?, effects, caps, policies, routing, defaults }`
- `ManifestPatch`：`{ base_manifest_hash, ops[], new_version? }`，支持 set/remove（merge 要求基于同一 base hash）
- `PatchMergeResult`：`{ patch, conflicts[] }`，冲突包含路径与涉及的 patch 索引
- `PatchConflict`：`{ path, kind, patches[], ops[] }`（kind: same_path/prefix_overlap）
- `Proposal`：`{ id, author, base_manifest_hash, manifest, status }`
- `GovernanceEvent`：`Proposed/ShadowReport/Approved/Applied`
- `RollbackEvent`：`{ snapshot_hash, snapshot_journal_len, prior_journal_len, reason }`
- `SnapshotCatalog`：`{ records[], retention }`
- `SnapshotRetentionPolicy`：`{ max_snapshots }`
- `AuditFilter`：`{ kinds?, from_time?, to_time?, from_event_id?, to_event_id?, caused_by? }`
