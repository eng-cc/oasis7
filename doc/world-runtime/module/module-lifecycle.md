# oasis7 Runtime：模块治理与生命周期（设计分册）

审计轮次: 4

本分册为 `doc/world-runtime/prd.md` 的详细展开。

## 模块治理与兼容性（草案）
- **版本与兼容**：`interface_version` 由内核维护；模块声明兼容范围，若不兼容则拒绝加载。
- **治理闭环**：模块变更走 `propose → shadow → approve → apply`，升级/回滚均形成审计事件。
- **沙箱限制**：内存上限、指令燃料（gas）、调用频率、输出/事件大小上限。
- **能力/政策**：模块不能直接 I/O，只能产出 `EffectIntent`，由 capability/policy 决定是否执行。
- **确定性约束**：禁止读取真实时间/随机数；非确定性来源必须通过 receipt 写回事件流。

## 模块注册表与存储（草案）

> 目标：用**内容寻址**与**审计元数据**管理 WASM 模块，支持可回放、可治理的动态装载。

**存储布局（示意）**
- `module_registry.json`：模块索引（哈希 → 元数据）
- `modules/<wasm_hash>.wasm`：WASM 工件（只读、内容地址）
- `modules/<wasm_hash>.meta.json`：模块元信息（manifest 快照）

**module_registry.json（示意结构）**
```
{
  "version": 1,
  "updated_at": 123,
  "records": [
    {
      "wasm_hash": "...",
      "module_id": "m.weather",
      "name": "Weather",
      "version": "0.1.0",
      "interface_version": "wasm-1",
      "kind": "Reducer",
      "registered_at": 120,
      "registered_by": "agent:alpha",
      "audit_ref": "event:1234"
    }
  ]
}
```

**modules/<wasm_hash>.meta.json（示意结构）**
```
{
  "module_id": "m.weather",
  "name": "Weather",
  "version": "0.1.0",
  "interface_version": "wasm-1",
  "kind": "Reducer",
  "wasm_hash": "...",
  "exports": ["reduce"],
  "subscriptions": ["WorldEvent/WeatherTick"],
  "required_caps": ["cap.weather.query"],
  "limits": { "max_mem_bytes": 1048576, "max_gas": 100000, "max_call_rate": 1 }
}
```

**ModuleRecord（索引条目）**
```rust
struct ModuleRecord {
    wasm_hash: String,
    module_id: String,
    name: String,
    version: String,
    interface_version: String,
    kind: ModuleKind,
    registered_at: i64,
    registered_by: String,   // agent_id / system
    audit_ref: String,       // 对应 RegisterModule 事件 id
}
```

**加载/缓存策略**
- **按哈希装载**：模块加载必须提供 `wasm_hash`，不允许同名替换。
- **LRU 缓存**：内存中缓存已编译模块（带 `max_cached_modules` 上限）。
- **冷启动**：按需从 `modules/` 读取工件；找不到则拒绝加载并记录事件。

## 模块注册 Happy Path（草案）

```
ArtifactWrite(wasm_hash) 
  -> ProposeModuleChangeSet(register+activate)
    -> ShadowReport(pass)
      -> Approve
        -> Apply
          -> RegisterModule event
          -> ActivateModule event
          -> module_registry.json 更新
```

## 模块注册 Failure Path（草案）

```
ArtifactWrite(wasm_hash)
  -> ProposeModuleChangeSet(register+activate)
    -> ShadowReport(failed)
      -> Reject
        -> 无 Apply（不写入模块事件/注册表）
```

```
ArtifactWrite(wasm_hash)
  -> ProposeModuleChangeSet(register+activate)
    -> ShadowReport(pass)
      -> Approve
        -> Apply
          -> ModuleCallFailed(reject_reason)
          -> 无 Register/Activate（不更新注册表）
```

## 模块治理流程接入（草案）

**流程（概要）**
1. Agent 编译模块 → 计算 `wasm_hash` → 将工件写入 `modules/<wasm_hash>.wasm`。
2. 生成 `ModuleManifest` 与变更计划（Register/Activate/Upgrade）。
3. `propose → shadow → approve → apply`：治理闭环审查模块变更。
4. `apply` 成功后写入 `RegisterModule/ActivateModule/UpgradeModule` 事件，并更新模块注册表。

**Shadow 校验（示意）**
- 工件存在性与哈希一致性校验（`wasm_hash`）。
- 接口版本与 ABI 兼容性校验（`interface_version`）。
- `required_caps` 与 `Policy` 规则校验。
- `limits` 合法性校验（不超过系统上限）。

**Apply 行为（示意）**
- 生成模块生命周期事件并追加到事件流。
- 更新 `module_registry.json` 与内存缓存索引。
- 若任何校验失败，拒绝 apply 并记录 `ModuleCallFailed`（带拒绝原因）。

**ModuleChangeSet（示意）**
```rust
struct ModuleChangeSet {
    register: Vec<ModuleManifest>,
    activate: Vec<ModuleActivation>,
    deactivate: Vec<ModuleDeactivation>,
    upgrade: Vec<ModuleUpgrade>,
}

struct ModuleActivation { module_id: String, version: String }
struct ModuleDeactivation { module_id: String, reason: String }
struct ModuleUpgrade { module_id: String, from_version: String, to_version: String, wasm_hash: String }
```

**Apply 事件序列（示意）**
1. `RegisterModule`（若有 register）
2. `UpgradeModule`（若有 upgrade）
3. `ActivateModule`（若有 activate）
4. `DeactivateModule`（若有 deactivate）
> 顺序固定以保证回放确定性；同类事件按 `module_id` 字典序处理。

**ModuleChangeSet 应用算法（草案）**
```
fn apply_module_changes(changes: ModuleChangeSet) -> Result<()> {
  validate_changes(changes)?;
  shadow_check(changes)?;

  // 1) register
  for m in sort_by_module_id(changes.register) {
    write_event(RegisterModule { ..m });
    registry.insert(m);
  }

  // 2) upgrade
  for u in sort_by_module_id(changes.upgrade) {
    write_event(UpgradeModule { ..u });
    registry.update(u);
  }

  // 3) activate
  for a in sort_by_module_id(changes.activate) {
    write_event(ActivateModule { ..a });
    registry.activate(a);
  }

  // 4) deactivate
  for d in sort_by_module_id(changes.deactivate) {
    write_event(DeactivateModule { ..d });
    registry.deactivate(d);
  }

  Ok(())
}
```

**ModuleChangeSet 校验规则（示意）**
- `module_id` 在 `register/activate/deactivate/upgrade` 内不得重复冲突。
- `register` 与 `upgrade` 不能同时针对同一 `module_id`。
- `activate` 与 `deactivate` 若同时出现，按顺序执行，但必须显式声明（避免隐式切换）。
- `upgrade` 需要 `from_version` == 当前激活版本；`to_version` 必须单调递增。
- `register` 后默认不自动激活，需显式 `activate`（保证可审计变更意图）。
- 所有变更均需通过 shadow 校验（hash/ABI/limits/caps）。

## ModuleChangeSet 在 Manifest/Patch 中的编码（草案）

**Manifest 扩展**
```rust
struct Manifest {
    reducers: Vec<ReducerSpec>,
    modules: Vec<ModuleManifest>,
    module_changes: Option<ModuleChangeSet>,
    effects: Vec<EffectSpec>,
    caps: Vec<CapabilityGrant>,
    policies: Vec<PolicyRule>,
    routing: RoutingSpec,
    defaults: Defaults,
}
```

**Patch 路径约定（示意）**
- `/modules`：替换模块清单（`set`）
- `/module_changes`：提交模块变更计划（`set`）

**Patch 示例（注册 + 激活）**
```
{
  "base_manifest_hash": "...",
  "ops": [
    { "op": "set", "path": "/modules", "value": [ ... ] },
    { "op": "set", "path": "/module_changes", "value": {
        "register": [ { "module_id": "m.weather", "version": "0.1.0", ... } ],
        "activate": [ { "module_id": "m.weather", "version": "0.1.0" } ],
        "deactivate": [],
        "upgrade": []
    } }
  ]
}
```

**Patch 示例（升级）**
```
{
  "ops": [
    { "op": "set", "path": "/modules", "value": [ ... ] },
    { "op": "set", "path": "/module_changes", "value": {
        "register": [],
        "activate": [],
        "deactivate": [],
        "upgrade": [
          { "module_id": "m.weather", "from_version": "0.1.0", "to_version": "0.2.0", "wasm_hash": "..." }
        ]
    } }
  ]
}
```

## ModuleChangeSet 生命周期（草案）

**提案阶段**
- `module_changes` 仅存在于提案的 manifest 中，用于描述预期模块变更。
- Shadow 阶段完成校验后生成 `ShadowReport`（包含错误/警告/通过项）。

**Apply 阶段**
- apply 时按事件序列写入生命周期事件，并更新注册表索引。
- apply 完成后，活动 manifest 中的 `module_changes` 应清空为 `null`（避免重复应用）。

**回放阶段**
- 注册表由事件流重建；`module_changes` 仅用于审计历史，不作为运行时指令。

## 多补丁冲突处理（草案）

> 目标：在 merge/patch 叠加时，确保模块变更的确定性与可审计性。

**冲突判定**
- 同一 `module_id` 在不同 patch 中出现 `register/upgrade/activate/deactivate` 任一重叠，即视为冲突。
- `register` 与 `upgrade` 对同一 `module_id` 视为硬冲突，必须人工选择。
- `activate` 与 `deactivate` 对同一 `module_id` 视为软冲突，允许选择保留其一。
- `upgrade` 的 `from_version` 不一致视为冲突。

**合并策略**
- 默认 **拒绝自动合并**，返回 `PatchMergeResult.conflicts` 供治理人工裁决。
- 若治理提供显式决策（保留 patch A 或 B），则按决策应用并记录审计事件。
- 合并后的 `ModuleChangeSet` 需重新通过 shadow 校验。

> V1 约定：治理“补丁”采用**完整 manifest 替换**语义（shadow 仅计算候选 manifest 哈希）。

## Manifest 版本与迁移策略（草案）

- `manifest_version`：每份 manifest 显式携带版本号（如 `v1` / `v2`）。
- **向后兼容**：新字段默认值由加载器填充（如 `module_changes = None`）。
- **向前拒绝**：内核遇到高于自身支持的版本时拒绝加载并记录审计事件。
- **迁移路径**：提供 `migrate_manifest(from, to)` 辅助函数；迁移需可确定性重放。
- **Patch 约束**：`ManifestPatch` 必须基于同一 `base_manifest_hash`，跨版本 patch 直接拒绝。

**加载/恢复流程（示意）**
```
load_manifest()
  -> if version == supported: use
  -> else if version < supported: migrate_manifest(version, supported) -> new_manifest
  -> else: reject + audit
```

**base_manifest_hash 行为**
- 迁移后生成新的 `manifest_hash`，用于后续 patch 基线。
- 迁移前后的哈希需记录在审计事件中，保证可追溯。

**迁移审计事件（草案）**
```
ManifestMigrated {
  from_version,
  to_version,
  from_hash,
  to_hash,
  reason,
}
```

## 稳定实例、交易与升级合同

- 活动模块以持久 `instance_id` 区分，同一 module artifact 可在不同 owner/target 下形成独立实例；确定性执行使用稳定实例顺序。
- `ModuleInstallTarget` 只允许 `SelfAgent` 与
  `LocationInfrastructure { location_id }`。target-aware install 及其
  finality 变体携带该字段；legacy install 归一化为 `SelfAgent`。location
  目标必须非空、存在且与 installer 当前 location 一致，失败路径不写状态。
- action、`ModuleInstalled` domain event、pending/release 状态与 installed
  instance 都持久化 `install_target`；历史缺失字段以 `SelfAgent` 为 serde
  默认值。snapshot/replay 不重新选择目标，基础设施实例继续使用已记录的
  infrastructure tick origin。
- deploy/register、install、activate、deactivate、upgrade、list/bid/purchase/delist/destroy 均通过 validated action -> domain event -> state apply 进入权威状态并可重放，不允许直接改 registry 或 instance cache。
- upgrade 保持 module identity，校验 owner、interface version、required exports、subscriptions、capability slots 与 limits；不兼容升级被拒绝且不产生部分事件。
- artifact 市场只允许当前 owner 上架、下架或销毁；结算时校验正价格、价格资源类型与余额，成交后转移 ownership 并清理相关 offer/bid。active instance 仍引用 artifact 时拒绝销毁。
- 费用事件只在成功动作路径产生；旧事件新增字段使用明确兼容默认值，replay 不重新撮合、不重新报价，也不把历史订单视为保证成交。
