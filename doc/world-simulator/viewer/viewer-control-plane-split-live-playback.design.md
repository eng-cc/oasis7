# Viewer 控制面：回放与 Live 分离设计

> 对应需求: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`
> 历史实施、验证与 task 状态：GitHub task issue evidence。

## 结构

1. **协议与握手层**：`ViewerControlProfile`、`PlaybackControl`、`LiveControl` 与 `HelloAck.control_profile` 让客户端知道服务端的动作集合。
2. **服务端与路由层**：playback 和 live 各执行所属控制；legacy `Control` 只桥接兼容请求。现有 legacy live handler 收到 seek 时记录并忽略，保持世界不回退；这不是 profile-specific 结构化响应。
3. **表现层**：timeline、egui、Web test API 与 automation 不把 live seek 暴露为可提交动作；当前 Web 控制入口对 seek 在发送前给出通用 unsupported-action 反馈，不能以“重连后再试”掩盖语义不支持。
4. **兼容层**：profile 未知时可临时显示 legacy 动作集合；若旧调用方仍发送 seek，live handler 的 log-and-ignore 行为必须被视为兼容性边界，而不是成功、断链或回退。
5. **观测层**：`logicalTime` 与 `eventSeq` 分别表达已观察的逻辑时间和事件顺序；`tick` 仅为前者的兼容 alias。控制请求、重连计时器与空 mailbox 不得合成任一观测增量。
6. **PromptControl authority 层**：增强 PromptControl 通过 negotiated capability 暴露 request identity、authoritative result、control-loss precedence、race/idempotency 与 runtime-instance application scope；它不改变 playback/live profile，也不新增 persistence/sync。

## PromptControl system contract

配对 PRD 的六个锚点是本 system leaf 的唯一技术合同入口：[`authority contract`](viewer-control-plane-split-live-playback.prd.md#prompt-control-authority-contract)、[`request identity`](viewer-control-plane-split-live-playback.prd.md#prompt-control-request-identity)、[`result status`](viewer-control-plane-split-live-playback.prd.md#prompt-control-result-status)、[`control-loss precedence`](viewer-control-plane-split-live-playback.prd.md#prompt-control-control-loss-precedence)、[`race and idempotency`](viewer-control-plane-split-live-playback.prd.md#prompt-control-race-and-idempotency)、[`application scope`](viewer-control-plane-split-live-playback.prd.md#prompt-control-application-scope)。PRD 锚点优先于本设计中的说明；产品承诺仍由 [`Agent 对话与 Prompt 控制`](../../product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md) 负责。

### Implementable spellings and recovery boundary

capability constant 固定为 `PROMPT_CONTROL_RESULT_CAPABILITY`，serialized token 为 `prompt_control_result_v1`，全局 `VIEWER_PROTOCOL_VERSION` 仍为 `2`。选中 capability 的 `HelloAck.authority_epoch: Option<String>` 提供 runtime namespace，成功且已授权的 `AuthoritativeRecoveryAck.binding_epoch: Option<u64>` 提供目标 Agent binding revision；两者都是 additive/default，legacy/未协商路径可省略，未授权错误不得暴露 binding epoch。实现 leaf 使用 `PromptControlResultStatus`、`PromptControlValueVisibility` 与 `PromptControlApplicationScope` 三个 `snake_case` serializable enum；对应 values 分别是 `accepted|applied|blocked|rejected|stale`、`hidden|metadata_only|latest_allowed` 与 `none|runtime_instance`。enhanced response/error 的字段名固定为 `request_id`、`authority_epoch`、`operation`、`preview`、`status`、`agent_id`、`player_id`、`session_epoch`、`binding_epoch`、`expected_version`、`version`、`digest`、`applied_fields`、`value_visibility`、`applied_scope`、`persistence_scope`、`sync_scope`、`reason_code`、`next_step`、`operation_digest`、`idempotent_replay`、`mutation_count` 和 `rolled_back_to_version`；新增字段仍为 optional/default。rollback wire 的 `to_version` 规范化为 digest preimage 的 `rollback_target`。

result ledger limits 固定在 `ViewerRuntimeLiveServerConfig` 的 `prompt_result_cache_capacity: usize = 256` 与 `prompt_result_receipt_max_bytes: usize = 65536`；无效零值拒绝启动/配置，字段不进入 Viewer wire 或 recovery persistence。`binding_epoch` 是按 Agent 的 runtime-only monotonic revision，命名空间属于当前 `authority_epoch`：新 instance 从空 map/`0` 开始，首次成功绑定为 `1`，每个成功 bind、unbind、release、transfer、force-rebind（含同 key）递增；unbind/release 后即使当前无绑定，也保留该 Agent 的最后 revision 作为 in-memory tombstone。旧 instance 请求先因 `expected_authority_epoch` 不匹配得到 hidden `blocked/result_unknown`，所以 restart 不需要持久化或重建 binding counter。

registration 使用既有 session-mutation rollback snapshot 时，snapshot 必须覆盖 binding maps 与 binding-revision map；persistence failure 恢复所有这些值，不得留下 revision advance。该恢复约束不修改 `recovery_receipt.rs`，不把 prompt profile、result ledger 或 authority epoch 加入持久化。

### 状态图与请求关联

`HelloV2` 仅在 runtime 支持完整合同时选择 additive capability `prompt_control_result_v1`。增强 `preview`、`apply`、`rollback` 请求携带 client-created `request_id`、`session_epoch`、`binding_epoch`、`expected_authority_epoch` 与 `expected_version`；这些字段与完整规范化 operation 一起进入 player proof。`operation_digest` 对 typed semantic operation 使用固定 domain-separated canonical-CBOR/BLAKE3-256 preimage，`request_id` 只作为 ledger key，不进入 digest。

authority 先解析 operation/request ID，再验证 proof、session/binding/epoch，之后查 fixed-capacity result ledger、消费 nonce、检查 expected version，最后 serialized commit。客户端先从 selected `HelloAck.authority_epoch`、再从成功授权 registration/binding `AuthoritativeRecoveryAck.binding_epoch` 取得 enhanced request 的两个 runtime values，不得本地猜测。可解析 request ID 的 structural malformed、unsupported、unknown-target 或 request-ID conflict 是 `rejected`；没有可解析 request ID 的 malformed request 只能返回 uncorrelated protocol error，不属于 W3 evidence。缺失或无效 proof 属于 hidden blocked/auth-control class，wire status 保持 `blocked`，reason code 为 `auth_required`/`auth_invalid`；session/binding loss 是 hidden `blocked/control_lost`，runtime epoch mismatch 是 redacted `blocked/result_unknown`。这些 auth/control blockers 都先于 expected-version 检查。有效 control 下的同 baseline 并发形成一个 `applied` 与一个 `stale`；control-loss race 形成 hidden `blocked/control_lost`，优先于旧 version。same-key retry replay 原结果且不重复 mutation；不同 digest 的同 request ID 返回 conflict；runtime restart 后旧 retry 为 hidden `blocked/result_unknown`，不重放。

### 结果可见性与应用边界

`accepted`、`applied`、`blocked`、`rejected`、`stale` 是唯一 enhanced statuses。`operation_digest` 在 visibility 允许时返回；`value_visibility=hidden` 的 `blocked/auth_required`、`blocked/auth_invalid`、`blocked/control_lost` 与 redacted `blocked/result_unknown` 省略 wire digest，仅授权 runtime trace 可保留。Preview 必须证明无 mutation；actual apply 的 scope tuple 固定为 `runtime_instance`、`none`、`none`（apply、persistence、sync），Viewer 不得由 ack、local snapshot、reconnect 或 reload 推导更大后果。

该合同只覆盖 AC-PROMPT-004、AC-PROMPT-007、AC-PROMPT-009 与 AC-PROMPT-011 的系统边界。它不引入 durable Prompt recovery、cross-session replication、cross-entry sync、production-supervisor activation 或新的 race endpoint。实现与证据分别由 protocol/runtime、Agent/runner、Viewer、QA leaf 消费，并在 W2 trusted merge 后才可发布 system leaf。

## 不变量

- live 的世界推进单调，不提供回退或跳时控制。
- `seek` 枚举可以为非 live 兼容保留，但不得泄漏为 live 玩家或自动化动作。
- Web 发送前拒绝与 legacy handler 的记录并忽略必须如实区分；两者都不等于 channel failure、发送成功或世界回退。
- queued/accepted 是本地发送阶段，不能替代 runtime 事件、snapshot 或 completion feedback；空结果必须保持空结果。
- PromptControl 的本地 accepted、cached replay 与 actual applied 必须按 authority result 区分；hidden/redacted result 不得把 digest 或 profile value 泄漏到 browser。
- PromptControl application scope 不越过 runtime instance；persistence 与 sync scope 必须保持 `none`。

## 代码与验证接点

- 协议/服务：`crates/oasis7_proto/src/viewer.rs`、`crates/oasis7/src/viewer/{protocol.rs,server.rs,live_controls.rs,runtime_live.rs}`。
- PromptControl authority 与签名：`crates/oasis7/src/viewer/auth.rs`、`crates/oasis7/src/viewer/runtime_live.rs`、`crates/oasis7/src/viewer/runtime_live/control_plane.rs`、`crates/oasis7/src/viewer/runtime_live/control_plane/prompt_profile.rs`、`crates/oasis7/src/viewer/runtime_live/llm_sidecar.rs`。
- 表现/自动化：`crates/oasis7_viewer/software_safe_src/{legacy_core.js,legacy_core_control_gate.test.js}`；生成浏览器入口为 `crates/oasis7_viewer/viewer.js`。
- PromptControl 浏览器签名与反馈 consumer 还需对齐 `crates/oasis7_viewer/software_safe_src/{main.jsx,viewer_feedback_module.js,viewer_auth_surface_module.js,software_safe_state.js}`；runner/QA 使用真实 backend-issued grant 与 runtime trace。
- 本设计不改变世界规则、P2P/共识回退策略、视觉方向或测试发布判断；实现 leaf 不得在此 system leaf 中合并。
