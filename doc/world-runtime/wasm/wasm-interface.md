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
