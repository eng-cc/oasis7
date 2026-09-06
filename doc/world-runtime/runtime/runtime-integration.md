# oasis7 Runtime：运行时集成要点（设计分册）

审计轮次: 4

本分册为 `doc/world-runtime/prd.md` 的详细展开。

## 模块加载与缓存（草案）

- **加载键**：仅允许按 `wasm_hash` 加载，拒绝同名覆盖。
- **缓存策略**：LRU + `max_cached_modules` 上限；超限时淘汰最久未使用模块。
- **编译缓存**：可选缓存已编译实例（WASM → 本地可执行表示）。
- **失败事件**：加载或执行失败统一写入 `ModuleCallFailed`（加载失败在调用阶段映射为 `SandboxUnavailable`）。

## 沙箱执行器与资源限制（草案）

- **资源限额**：`max_mem_bytes`、`max_gas`、`max_call_rate`、`max_output_bytes`。
- **隔离**：模块不可直接访问 I/O，仅能产生 `EffectIntent`；沙盒不做语义/策略限制，仅负责隔离与计量。
- **失败处理**：执行失败统一触发 `ModuleCallFailed`；错误码使用 `ModuleCallErrorCode`（如 `Trap`、`Timeout`、`OutOfFuel`、`Interrupted`、`OutputTooLarge`、`EffectLimitExceeded`、`EmitLimitExceeded`、`CapsDenied`、`PolicyDenied`、`SandboxUnavailable`、`InvalidOutput`）。
- **执行入口**：`World::execute_module_call` 通过沙箱接口执行模块并返回 `ModuleOutput`。
- **最小执行 ABI**：导出 `memory`/`alloc`/`reduce|call`，`reduce/call(i32, i32) -> (i32, i32)` 返回输出指针与长度（入口取决于 ModuleKind，输出使用 Canonical CBOR 反序列化）。
- **输入编码**：事件/动作输入使用 Canonical CBOR 编码，封装在 `ModuleCallInput { ctx, event|action }` 中，确保可回放与确定性。
- **配置哈希**：`ModuleContext.world_config_hash` 采用当前 manifest 哈希，便于模块检测配置变更。
- **模块状态**：reducer 调用会携带 `state`（空字节串代表无历史状态）；`new_state` 会触发 `ModuleStateUpdated` 事件并写回状态，确保回放一致；pure 模块返回 `new_state` 视为 InvalidOutput。

### ModuleCallFailed 错误码映射（当前实现）
- `SandboxUnavailable`：加载工件或沙箱不可用（如按 `wasm_hash` 加载失败）。
- `Trap` / `Timeout` / `OutOfFuel` / `Interrupted`：沙箱调用层透传失败。
- `OutputTooLarge` / `EffectLimitExceeded` / `EmitLimitExceeded`：`ModuleOutput` 校验超限。
- `CapsDenied`：`cap_ref/cap_slot/required_caps` 校验失败。
- `PolicyDenied`：策略模块拒绝或策略 hook 执行失败。
- `InvalidOutput`：输出编码/结构非法，或 pure 模块返回 `new_state`。

## LLM 驱动与 Agent 内部模块（设计补充）

连续世界 Agent 的异步 scheduler、MVCC decision envelope、durable cognition
journal、recovery 与 continuation 权威合同见
`doc/world-runtime/runtime/agent-cognition-lifecycle.prd.md` 及同名 design；
Agent-owned session/turn/provider/context/memory/goal 合同见
`doc/world-simulator/llm/continuous-agent-harness.prd.md`。本节保留集成摘要，
不重复定义上述专题的 identity、状态机或恢复语义。

- **LLM 驱动**：Agent 决策由 LLM 执行，推理服务采用 OpenAI 兼容 API（endpoint/model/auth/超时/重试/预算可配置）。
- **确定性与回放**：LLM 调用视为外部效应；运行时需记录输入/输出或最终决策事件以保证回放一致性（回放时不再次调用 LLM）。
- **Memory Module**：Agent 记忆策略封装为独立 WASM 模块 + 受限存储配额；由 Agent runtime 触发调用，负责写入 observation/event/action_result 并生成上下文摘要。
- **可演化**：Memory module 与其它内部模块遵循同一治理/升级流程，可由 Agent 自主更新记忆策略。

## Capability/Policy 绑定（草案）

- **绑定点**：模块输出的每个 `EffectIntent` 必须关联 `cap_ref`。
- **校验**：`required_caps` 与系统 grants 匹配；policy 允许才执行。
- **审计**：策略拒绝记录为 `PolicyDecisionRecorded`。

## 事件订阅与路由（草案）

- **订阅来源**：`ModuleManifest.subscriptions` 指定 event/action kinds。
- **路由顺序**：按 `instance_id` 字典序调用，保证确定性。
- **调度主键**：模块状态读写与 trace key 以 `instance_id` 为主；仅在 legacy 模块无实例记录时回退到 `module_id`。
- **Action 路由**：`action.*` 订阅支持 `pre_action/post_action` 阶段：
  - `pre_action`：规则模块先行校验/计费/覆盖动作参数。
  - `post_action`：动作应用后派发衍生事件或效果。
- **Event 路由**：`event.*` 订阅在事件追加后触发（`post_event`），用于捕获已落盘事件。
- **阶段默认值**：`event_kinds` 默认 `post_event`；`action_kinds` 默认 `pre_action`；二者同时存在视为无效订阅。
- **订阅校验**：`post_event` 不允许配置 `action_kinds`，`pre_action/post_action` 不允许配置 `event_kinds`。
- **输入结构**：`pre_action` 仅传入 action；`post_action` 同时传入 action + 结果事件；`post_event` 仅传入 event。阶段信息通过 `ctx.stage` 传递。
- **隔离性**：模块之间不共享状态，状态由 reducer 自身维护。
- **事件 kind 命名**：`domain.agent_registered`、`domain.agent_moved`、`domain.action_rejected` 等；其它系统事件使用 `effect.*`/`module.*`/`snapshot.*`/`manifest.*` 前缀。

## 模块输出校验（草案）

- **数量限制**：`effects`/`emits` 数量不得超过 `ModuleLimits`。
- **大小限制**：`ModuleOutput` 编码后大小不得超过 `max_output_bytes`。
- **拒绝策略**：违反规则写入 `ModuleCallFailed` 并丢弃输出。

## 运行时接口（草案）
- `World::step(n_ticks)`：推进时间并处理事件队列
- `World::step_with_modules(sandbox)`：推进时间并处理事件队列，同时路由 action/event 到模块
- `World::apply_action(action)`：校验与入队事件
- `World::emit_effect(intent)`：校验 capability + policy → 入队
- `World::ingest_receipt(receipt)`：写入事件流并唤醒等待
- `World::snapshot()` / `World::restore()`：快照与恢复
- `World::create_snapshot()`：创建并记录快照
- `World::set_snapshot_retention(policy)`：快照保留策略
- `World::save_snapshot_to_dir(dir)` / `World::prune_snapshot_files(dir)`：快照文件落盘与清理
- `World::save_to_dir_with_modules(dir)`：保存 world + 模块存储
- `World::load_from_dir_with_modules(dir)`：加载 world + 模块存储
- `World::propose_manifest_update(manifest)`：提交治理提案
- `World::propose_manifest_patch(patch)`：以 patch 形式提交治理提案
- `World::shadow_proposal(id)`：影子运行并生成候选 hash
- `World::approve_proposal(id)`：审批或拒绝
- `World::apply_proposal(id)`：应用并更新 manifest
- `World::rollback_to_snapshot(snapshot, journal)`：回滚并记录审计事件
- `World::audit_events(filter)`：审计筛选（按类型/时间/因果）
- `World::save_audit_log(path, filter)`：导出审计事件到文件
- `diff_manifest(base, target)` / `merge_manifest_patches(base, patches)` / `merge_manifest_patches_with_conflicts(...)`：diff/merge 辅助
- `Scheduler::tick()`：按确定性顺序调度 agent cells
- `World::register_module_artifact(wasm_hash, bytes)`：写入模块工件
- `World::load_module(wasm_hash)`：按哈希加载模块（命中缓存或从工件库读取）
- `World::set_module_cache_max(max_cached_modules)`：调整模块缓存容量
- `World::set_module_limits_max(limits)`：调整模块资源上限
- `World::execute_module_call(module_id, trace_id, input, sandbox)`：执行模块调用并写入 ModuleCallFailed/ModuleEmitted
- `World::route_action_to_modules(envelope, sandbox)`：按订阅路由 action 并触发模块调用
- `World::route_action_to_modules_with_stage(envelope, stage, sandbox)`：按订阅与阶段路由 action
- `World::route_event_to_modules(event, sandbox)`：按订阅路由事件并触发模块调用
- `World::propose_module_changes(changes)`：提交模块变更提案（治理闭环）
- `World::module_registry()`：读取模块索引

## 代码结构调整（草案）

- `Manifest` 当前结构为 `{ version, content }`；`module_changes` 作为 `content` 下可选键（而非顶层强类型字段）。
- `ManifestPatch` 允许 `"/module_changes"` set/remove
- `GovernanceEvent::Applied` 当前字段为 `manifest_hash`/`consensus_height`/`threshold`/`signer_node_ids`（不含 `module_changes`）。
- `module_changes` 在提案应用时通过 `ModuleEvent` 序列落盘，再由 `ManifestUpdated` 写入去除 `module_changes` 的目标 manifest。
- 审计导出支持 `module_call_failed` 与 `governance` 记录
