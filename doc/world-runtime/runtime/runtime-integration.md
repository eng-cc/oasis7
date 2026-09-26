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
  - “已落盘”保留旧集成摘要用语；current `post_event` 表示 canonical append/routing 阶段，单凭该阶段不证明 filesystem generation 已 sync/commit，durability 仍按 root/storage 合同验证。
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

## 集成合同的适用设计视图
owner runtime_engineer；本分册是 active runtime integration design，旧“草案”标题和接口清单保留其历史/混合成熟度，不宣告每个接口已持久完成。审读输入 eng-cc/oasis7@f9d5a552d9af04c1b1398262808198a58e560230，当前实现以对应源码入口为准，测试定义不等于已运行。目标是 sandbox/module/LLM 输出经统一 Kernel/proof/transaction 边界；非目标是重定义 ABI、Agent cognition、BFT 或消费者 DTO。

<a id="integration-module-boundary"></a>
### Module 输入输出与外部 authority
wasm_hash 唯一加载键、cache/LRU、Canonical CBOR、memory/alloc/reduce/call、输出 limits、pure new_state 拒绝及全部错误码仍按旧文。wire/default/extension/schema 的权威为 [WASM interface](../wasm/wasm-interface.md)、[executor PRD](../wasm/wasm-executor.prd.md)；sandbox 只隔离计量，不赋资源/策略/最终性。ModuleContext 的当前 manifest hash 是已选版本的上下文，不是客户端选版权。历史调用需要 [root version 设计](../design.md#runtime-version-design) 的原 block manifest/artifact binding，missing/mismatch 失败关闭；cache hit/编译成功不证明 activation。epoch/watchdog 本地安全界不变成 consensus clock。
Action/Event 从 envelope/ctx.stage 输入，按 instance_id 确定排序/隔离，legacy module_id fallback 只用于旧无实例记录。pre_action/post_action/post_event 及无效 subscriptions 规则保持原文。旧 post_event “已落盘”指 canonical event append/routing 后的阶段；没有 filesystem generation proof 时不能据此声明 fsync/durable commit。output/cap_ref/required_caps/policy/limits preflight 在 root stage；失败丢弃业务输出，既有单独 ModuleCallFailed audit 的有界顺序保持 root §5/6 原文，不扩展为完整 receipt 原子性。

<a id="integration-state-persistence"></a>
### 状态、事务与持久化
reducer state/new_state、module instance/registry、scheduler wake、pending effect 是不同持久事实；输入输出是候选，Kernel apply 后才为 canonical。目标统一 buffer 同时 prepare module state、费用、effects/emits、allocator/queue/journal/consensus，publish 前不改 live state；当前 direct/trusted 和 governed proposal typed stages 已有有界保证，step cloned boundary/其他生命周期/receipt/outbox/generation 整体仍 partial。事务失败不能消费 queue/费用/ID，infrastructure failure 不伪装业务拒绝。持久化用 [root generation/replay 合同](../design.md#runtime-pending-design) 及 ModuleStore integrity；replay 原事件/state 不重新调用 LLM，外部 effect 由目标 durable outbox postcommit 投递。通用 signed pending/lineage/service schemas 未完成，现有 effect queue 不能代签。
LLM observation/decision/provider/context/memory/goal 依赖原 cognition/harness authority，推理失败/accepted acknowledgement 不产生世界效果。消费者受 [pending](../design.md#runtime-pending-design)、[lineage](../design.md#runtime-lineage-design)、[service](../design.md#runtime-recovery-design)、[world scope](../design.md#runtime-world-scope-design) 约束，只以 committed receipt 改结论，本节不新增 DTO/UI。

<a id="integration-governance-api"></a>
### 当前入口、失败与兼容
接口表的 World::apply_proposal(id) 是现有 local-policy-gated wrapper；explicit certificate API 是 World::apply_proposal_with_finality(proposal_id,&certificate)，见 [治理设计](../governance/zero-trust-governance-receipt-hardening-2026-02-26.design.md#hardening-governance-boundary)。旧零信任两参数 apply_proposal 是历史 proposal spelling。Manifest version/content/module_changes 与 Applied 现有字段保持不变。signature/threshold/artifact/output 错误保留优先级、HMAC/legacy replay 及 ABI 默认兼容；不得隐式降级 missing authority/artifact。容量由 ModuleLimits/queue/profile authority 控制，cache eviction 不驱动世界进度，审计 errors 不包含额外敏感 payload。target 接口/activation/full durable 迁移需要 runtime/WASM/Agent 联审与 replay/failure evidence；风险是旧 draft 与 current 入口混淆，输入/ABI/消费者变更触发复核。

### 2.1 需求承接与分配表

输入身份 eng-cc/oasis7@f9d5a552d9af04c1b1398262808198a58e560230；新增 local anchors 是本文技术接受关系，非机器 schema。每行范围独立，外部未决保留。

| 上游 requirement / acceptance | 具体 obligation 与条件 | 本设计条款 | 外部 owner / dependency | 排除或未覆盖 |
| --- | --- | --- | --- | --- |
| [条款](../prd.md#runtime-deterministic-acceptance) | hash load/CBOR/limits/pure-state/output/cap/policy/instance排序，missing/mismatch零业务效果 | [设计](#integration-module-boundary) | WASM ABI/executor；runtime Kernel | cache/ModuleCallFailed不等于durable receipt |
| [条款](../prd.md#runtime-version-acceptance) | 历史governing version/artifact输入不可由cache/client选择 | [设计](#integration-module-boundary) | WASM compatibility与P2P proof | activation历史未proved |
| [条款](../prd.md#runtime-pending-acceptance) | LLM/effect候选不等于world effect，no recall replay及pending真实处置 | [设计](#integration-state-persistence) | Agent cognition/harness；P2P/consumer | 通用pending DTO未落地 |
| [条款](../prd.md#module-store-persistence-and-recovery-contract) | module state/registry/artifact/cache完整 hydration，mixed generation拒绝 | [设计](#integration-state-persistence) | runtime storage；WASM identity | whole root/outbox仍partial |
| [条款](../governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md#retained-active-hardening-contract-与历史-provenance) | wrapper与explicit finality API区分，ordered module→manifest→Applied | [设计](#integration-governance-api) | runtime governance；P2P finality；WASM | 历史两参数spelling非current API |

### 11.1 验证映射表

所有下列行为验证在本次文档编辑中未运行。定义/计划与当前实现、实际执行、发布分别成立；执行须另固定 source/integration/tested tree、config/world/entry/environment/window、exit/result/artifacts，并回 GitHub task evidence。target场景尚无完整runner时明确保持待证明，现有test/manual只是有界接收入口，不能伪称已实现或通过。

| 上游 requirement / acceptance | 本设计条款 | 独立 obligation / 条件 | 验证 source / ID、层级及候选环境 | 证据目标 | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [条款](../prd.md#runtime-deterministic-acceptance) | [设计](#integration-module-boundary) | hash load/CBOR/limits/pure-state/output/cap/policy/instance排序，missing/mismatch零业务效果 | [现有 test/manual](../../../crates/oasis7/src/runtime/tests/modules_permissions_policy_hooks.rs)；required 局部 permit/deny/context hash；target同manifest/parent输出越限/pure-state拒绝和全部stage/default/subscription invalid，记录root/fee/event | 未运行；未来同候选 log/root/receipt/metrics或consumer artifact入task evidence，QA判定 | cache/ModuleCallFailed不等于durable receipt |
| [条款](../prd.md#runtime-version-acceptance) | [设计](#integration-module-boundary) | 历史governing version/artifact输入不可由cache/client选择 | [现有 test/manual](../../../crates/oasis7/src/runtime/tests/module_action_loop_release_controls.rs)；full target activation窗口+missing历史工件，局部release control定义，不执行 | 未运行；未来同候选 log/root/receipt/metrics或consumer artifact入task evidence，QA判定 | activation历史未proved |
| [条款](../prd.md#runtime-pending-acceptance) | [设计](#integration-state-persistence) | LLM/effect候选不等于world effect，no recall replay及pending真实处置 | [现有 test/manual](../../../crates/oasis7/src/runtime/tests/execution_transaction_regressions.rs)；required局部queue/ingest rollback；target provider outage/restart/replay无调用/费用/效果及committed-only consumer | 未运行；未来同候选 log/root/receipt/metrics或consumer artifact入task evidence，QA判定 | 通用pending DTO未落地 |
| [条款](../prd.md#module-store-persistence-and-recovery-contract) | [设计](#integration-state-persistence) | module state/registry/artifact/cache完整 hydration，mixed generation拒绝 | [现行 manual](../../../testing-manual.md)；精确局部 source ../../../crates/oasis7/src/runtime/world/module_store_load_transaction_regressions.rs（定义/入口，非执行证据）；required late record/missing/tamper/no-store兼容局部定义；full target generation/crash/历史binding | 未运行；未来同候选 log/root/receipt/metrics或consumer artifact入task evidence，QA判定 | whole root/outbox仍partial |
| [条款](../governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md#retained-active-hardening-contract-与历史-provenance) | [设计](#integration-governance-api) | wrapper与explicit finality API区分，ordered module→manifest→Applied | [现有 test/manual](../../../crates/oasis7/src/runtime/tests/governance.rs)；required governance_policy_blocks_local_apply_proposal_path 定义；target复核调用边界/invalid certificate零部分apply | 未运行；未来同候选 log/root/receipt/metrics或consumer artifact入task evidence，QA判定 | 历史两参数spelling非current API |
