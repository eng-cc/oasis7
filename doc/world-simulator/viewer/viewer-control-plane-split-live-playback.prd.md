# Viewer 控制面：回放与 Live 分离

> 本文是回放/Live 控制 profile 与 live 无 seek 语义的当前专业 authority。它收敛两个 2026-02 源三件套；历史变更仅从 Git 与 GitHub task evidence 追溯。

- 对应设计: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.design.md`
- 历史实施、验证与 task 状态：GitHub task issue evidence。

## 目标

将 Viewer 控制语义分为 playback 与 live 两个 profile，使 live 世界只单调前进，不能把回放专属的 `seek` 误作连接失败或可回退操作。

## 范围

- 覆盖 profile、握手、控制请求路由、legacy bridge、timeline、automation 与 Web test API 的动作集合。
- 不重写世界规则、共识/P2P 语义、非 live 历史浏览、视觉方向或完整 timeline 架构。

## 接口 / 数据

- `ViewerControlProfile::{playback, live}`、`PlaybackControl`、`LiveControl`。
- `ViewerRequest::{PlaybackControl, LiveControl}` 与 `HelloAck.control_profile`。
- `window.__AW_TEST__` 的 `controlProfile`、动作描述/发送接口，以及 legacy live handler 的无回退行为。

## 当前合同

| profile | 支持控制 | 不支持控制 | 表现要求 |
| --- | --- | --- | --- |
| `playback` | `play`、`pause`、`step`、`seek` | — | timeline 与 automation 可按 playback 暴露 seek |
| `live` | `play`、`pause`、`step` | `seek` / `seek_event` | 当前 Web 控制入口在发送前以通用 unsupported-action 反馈拒绝 seek；legacy live request 收到 seek 时记录并忽略，不回退世界，也不把它报告为连接失败 |

- `HelloAck.control_profile` 是握手后的控制面真值；`ViewerRequest` 分别使用 `PlaybackControl` 与 `LiveControl`。
- 握手前可暂按 legacy 能力展示；当前 Web 入口不提供 seek 提交，未知或 live profile 的 seek 以发送前通用 unsupported-action 反馈拒绝。
- legacy `Control` 仅是兼容桥接；若旧调用方仍向 live handler 发送 seek，handler 只记录并忽略该请求，世界保持单调前进。
- live 禁止 seek 只适用于 live 服务，不改变非 live 的历史浏览/回放语义。

<a id="prompt-control-authority-contract"></a>
### PromptControl authority contract

- `prompt_control_result_v1` 是增强 PromptControl 结果合同的协商 capability；通过 `HelloV2` 选择，不改变全局 Viewer protocol version。只有实现完整合同并显式协商后，请求才属于 W3 证据；旧客户端继续走可兼容的 legacy 行为，但不能据此声称 W3 结果。
- 既有 serialized runtime authority 是唯一应用来源。客户端本地发送、accepted/queued、ack 到达、重连和本地 snapshot 都不等于 runtime 已应用。产品承诺与四个目标验收项见 [`Agent 对话与 Prompt 控制`](../../product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md) 的 AC-PROMPT-004、007、009、011；本合同只规定系统边界与可观察结果。
- 配对设计中的状态图与证据示例见 [`viewer 控制面设计`](viewer-control-plane-split-live-playback.design.md)。本节以下六个锚点是本专业 authority 的可消费合同。

#### Frozen wire and runtime spellings

- capability constant 使用 `PROMPT_CONTROL_RESULT_CAPABILITY`，其 `HelloV2` serialized token 固定为 `"prompt_control_result_v1"`；`VIEWER_PROTOCOL_VERSION` 保持 `2`。
- enhanced epoch 的 bootstrap/readback wire fields 固定为 `HelloAck.authority_epoch: Option<String>` 与 `AuthoritativeRecoveryAck.binding_epoch: Option<u64>`，均 additive/default 以保持 legacy decode。选中 `prompt_control_result_v1` 的 `HelloAck` 必须提供当前 `authority_epoch`；成功且已授权的目标 Agent registration/binding ack 必须提供该 Agent 当前 `binding_epoch`。未协商 capability 的握手与旧 registration response 可省略这些字段；这不改变全局 protocol version，也不向未授权错误暴露 binding epoch。
- enhanced result 的 Rust enum 名称与 wire values 固定为 `PromptControlResultStatus::{Accepted, Applied, Blocked, Rejected, Stale}` → `accepted`、`applied`、`blocked`、`rejected`、`stale`；`PromptControlValueVisibility::{Hidden, MetadataOnly, LatestAllowed}` → `hidden`、`metadata_only`、`latest_allowed`；`PromptControlApplicationScope::{None, RuntimeInstance}` → `none`、`runtime_instance`。三个 scope 字段均使用 `PromptControlApplicationScope` 的 snake_case 序列化值。
- enhanced response/error 的 wire field names 固定为 `request_id`、`authority_epoch`、`operation`、`preview`、`status`、`agent_id`、`player_id`、`session_epoch`、`binding_epoch`、`expected_version`、`version`、`digest`、`applied_fields`、`value_visibility`、`applied_scope`、`persistence_scope`、`sync_scope`、`reason_code`、`next_step`、`operation_digest`、`idempotent_replay`、`mutation_count` 与既有 `rolled_back_to_version`；新增字段继续使用 optional/default wire compatibility。
- existing `PromptControlOperation` 与 request wire field `to_version` 保持不变；canonical operation preimage 的 `rollback_target` 是从 `to_version` 规范化得到的 typed value，不能另造第二个 rollback target 字段。
- result ledger limits 属于 `ViewerRuntimeLiveServerConfig`，字段固定为 `prompt_result_cache_capacity: usize`（default `256`）与 `prompt_result_receipt_max_bytes: usize`（default `65536`）。两者为正值才可启动/接受配置；它们不出现在 Viewer wire，也不进入 recovery persistence。

<a id="prompt-control-request-identity"></a>
### PromptControl request identity

- enhanced `preview`、`apply`、`rollback` 请求的新增 DTO 字段都必须是 additive optional/default，以保持旧 JSON 客户端可解码。协商 capability 后，`request_id`、`session_epoch`、`binding_epoch`、`expected_authority_epoch` 和非空 `expected_version` 均为必需；缺失字段返回稳定的 `prompt_control_field_required`，不产生 mutation。未协商增强 capability 的客户端不能获得增强语义。

| 字段 | enhanced 规则与合同含义 |
| --- | --- |
| `request_id: Option<String>` | 非空 opaque identity，最多 128 个 UTF-8 bytes；同一逻辑操作的 transport retry 必须复用它。 |
| `session_epoch: Option<u64>` | 当前 player session 的 authoritative revision；revoke/rotate 后递增。 |
| `binding_epoch: Option<u64>` | Agent/player/key binding revision；bind、unbind、release、transfer、force-rebind（包括同 key 变化）后递增。 |
| `expected_authority_epoch: Option<String>` | 启动时生成且不持久化的 runtime-instance epoch；变化后不得把旧 ledger 当作当前结果。 |
| `expected_version: Option<u64>` | 草稿所依据的 profile version；preview 也必须携带，null 不表示 latest。 |
| operation payload | 既有 patch/rollback 字段；先按既有 trim/empty 规则规范化，mutating apply/rollback 的 `updated_by` 必须等于 `player_id`。 |

- player proof 必须覆盖完整 enhanced operation：`request_id`、Agent/player identity、以上三个 epoch、`expected_version`、operation/mode、`preview`、规范化 patch、rollback target 和 `updated_by`。hosted strong-auth grant 必须由真实 backend 签发并继续绑定 action/player/key/Agent；它不替代 player proof。
- enhanced client 必须先从选中 capability 的 `HelloAck.authority_epoch` 捕获当前 runtime namespace，再从针对目标 Agent 的成功授权 `AuthoritativeRecoveryAck.binding_epoch` 捕获当前 binding revision；客户端不得猜测或从本地 snapshot/ack arrival 推导任一 epoch。重启以新的 authority epoch 使旧 request 失效，rebind/bind transition 以新的 binding epoch 使旧 binding identity 失效；缺失 bootstrap/readback 时不能构造 W3 enhanced request。
- `binding_epoch` 是 authority 内按 Agent 维护的 in-memory monotonic revision，并与 `authority_epoch` 成对解释。新 server instance 生成新的 `authority_epoch`、空 ledger 与空 binding-revision map；首次观察到的 Agent 无 revision 时为 `binding_epoch=0`，首次成功建立绑定为 `1`，之后每次成功 bind、unbind、release、transfer 或 force-rebind（包括同 key 变化）递增一次；unbind/release 后即使当前无绑定，也保留该 Agent 的最后 revision 作为 in-memory tombstone。重启后计数器可从 `0` 重新开始，因为旧请求首先因 `expected_authority_epoch` 不匹配而得到 hidden `blocked/result_unknown`，不存在跨 instance 的 epoch 比较或恢复持久化。
- 支持 backend registration 的同一 session-mutation rollback snapshot 必须同时保存 binding maps 与 binding-revision map。若 registration persistence 失败，必须把 session、binding、key map 与 revision counter 全部恢复到提交前值；失败尝试不得留下 binding epoch advance。`recovery_receipt.rs` 与 prompt profile/result ledger durability 均不因本合同扩展。
- `operation_digest` 是 typed semantic operation 的 canonical digest，而不是 raw JSON：

```text
operation_digest = h_v1(
  "oasis7.viewer.prompt-control.operation.v1",
  PromptControlOperationIdentityV1 {
    operation, preview, agent_id, player_id,
    session_epoch, binding_epoch, expected_authority_epoch,
    expected_version,
    system_prompt: PatchField::{unchanged|clear|set(utf8_string)},
    short_term_goal: PatchField::{unchanged|clear|set(utf8_string)},
    long_term_goal: PatchField::{unchanged|clear|set(utf8_string)},
    rollback_target: Option<u64>, updated_by
  }
)
```

  `h_v1` 使用仓库已有 domain-separated canonical-CBOR `[domain,payload]` 与 BLAKE3-256；字段顺序、UTF-8 bytes、trim/empty normalization 以及 unchanged/clear/set 的类型区分都固定。auth nonce/signature 与 grant transport timestamps 不进入 digest，`request_id` 作为 logical-operation key 也不进入 digest。当前签名 consumer 见 `crates/oasis7/src/viewer/auth.rs:124-148,834-872`，浏览器构造入口见 `crates/oasis7_viewer/software_safe_src/legacy_core.js:1649-1692`。

<a id="prompt-control-result-status"></a>
### PromptControl result status

- enhanced result 的 `status` 只能是 `accepted`、`applied`、`blocked`、`rejected` 或 `stale`；可解析请求应 echo `request_id`，并按 authority 结果提供 `authority_epoch`、身份/epoch、版本/摘要、`value_visibility`、scope tuple、`reason_code`/`next_step`、`operation_digest`、`idempotent_replay` 与 `mutation_count`。
- `accepted` 可表示无 mutation 的 Preview 或 authenticated verified no-op；AC-PROMPT-004 的 Preview 必须明确 `preview=true`，scope 全为 `none`，并以 profile/event count 证明未 mutation。`applied` 只表示 authority 已 commit，并携带 `mutation_count=1` 与 runtime-instance scope。`blocked` 表示 authority 未应用；`auth_required`、`auth_invalid`、`control_lost`、`result_unknown` 是稳定 reason code，并按隐藏策略返回；缺失/无效 proof 属于 hidden blocked/auth-control class，不能仅因 proof malformed 改成 `rejected`。`stale` 表示控制仍有效但 commit 时 version/digest 不匹配；旧 patch 不应用。可解析 request ID 的 malformed/unsupported/unknown-target/request-ID conflict 是 `rejected`，不能 silent retry；没有可解析 request ID 的 malformed request 只能返回 uncorrelated protocol error，不属于 W3 evidence。
- `value_visibility` 由 authority 设置为 `hidden`、`metadata_only` 或 `latest_allowed`，不能由 client 选择。`operation_digest` 在结果可见性允许时必须出现在 wire result；`value_visibility=hidden`（包括 `blocked/control_lost` 与 redacted `blocked/result_unknown`）必须省略 wire digest，只能保留在 authorized runtime trace。browser error 不得暴露该 trace。

<a id="prompt-control-control-loss-precedence"></a>
### PromptControl control-loss precedence

每个 enhanced operation 都由既有 serialized runtime authority 按以下顺序评估：解析 operation 与 request ID；验证 player proof；校验 session key/`session_epoch`；校验 Agent/player/key binding/`binding_epoch`；校验 `expected_authority_epoch`；查询 bounded result ledger；消费 fresh auth nonce 并校验 grant/operation；最后在 commit 前校验 `expected_version` 与当前 digest。可解析 request ID 的 malformed/unsupported operation 返回 `rejected`；缺失或无效 player proof 属于 hidden blocked/auth-control class，wire status 保持 `blocked`，reason code 分别为 `auth_required`/`auth_invalid`，不得仅因 proof malformed 就改成 `rejected`；session 或 binding 丢失返回 hidden `blocked/control_lost`；runtime epoch 变化返回 redacted `blocked/result_unknown`。这些 auth/control blockers 均先于 expected-version 检查，控制丢失优先于值变化。

`blocked/auth_required` 与 `blocked/auth_invalid` 使用同一 auth privacy boundary：仅保留可安全关联的 `request_id`（可解析时）、已知 `agent_id`、`status=blocked`、对应 reason code、generic re-auth `next_step` 与可用的 `authority_epoch`；必须 `value_visibility=hidden`，省略 player/session/binding/version/profile/diff/applied fields、digest、persistence/sync scope 与 retry/transfer action，且不得消费 nonce 或 mutation。`blocked/control_lost` 仅保留可安全关联的 `request_id`（可解析时）、已知 `agent_id`、`status=blocked`、`reason_code=control_lost`、generic re-auth/rebind `next_step` 与可用的 `authority_epoch`；必须 `value_visibility=hidden`，省略 player/session/binding/version/profile/diff/applied fields、digest、persistence/sync scope 与 retry/transfer action，且不得 mutation。`blocked/result_unknown` 同样 redacted、`value_visibility=hidden`，省略 wire `operation_digest`，不得 retry 或 mutation；authorized runtime trace 可保留 digest。

<a id="prompt-control-race-and-idempotency"></a>
### PromptControl race and idempotency

- AC-PROMPT-009 必须在 evidence 中区分两种真实并发：同一 expected version 与有效 binding/session epochs 的两次 authorized request 产生一个 `applied` 和一个 `stale`；若 force-rebind/revoke 在 A 的 revalidation/commit 前由 B 提交，A 必须产生一个 hidden `blocked/control_lost`，即使 A 的 expected version 也旧。两者都必须以 runtime trace 证明 baseline、lock/commit order 和 browser/result correlation。
- 不增加 production barrier、direct sidecar mutation、hidden scheduler mode 或 unauthenticated race endpoint。两名真实 authorized clients 与 runtime lock/serialization 是默认路径；如需确定性调度，只能使用 runner-side 或明确 test-only instrumentation。
- mutation idempotency key 固定为 `(authority_epoch, authenticated_player_id, request_id)`。runtime-only ledger 使用 fixed-capacity、non-evicting 配置 `prompt_result_cache_capacity=256` 与 `prompt_result_receipt_max_bytes=65536`；zero/invalid 配置 fail closed，禁止 LRU/silent eviction。新请求在 cache full 时返回 `blocked/result_cache_full`，oversize receipt 返回 `rejected/result_receipt_too_large`，两者都在 nonce consumption/mutation 前发生。
- cache lookup 必须在 cryptographic verification、principal/session/binding/epoch checks 后、auth nonce consumption 前进行。同 key 同 digest replay 原结果并标记 `idempotent_replay=true`，不同 digest 返回 `rejected/request_id_conflict`；session/binding loss 必须先于 cached success redaction。runtime restart 生成新 authority epoch 与空 ledger，旧 retry 返回 hidden `blocked/result_unknown`，不重新执行。

<a id="prompt-control-application-scope"></a>
### PromptControl application scope

- W3 的 actual apply 只承诺 `applied_scope=runtime_instance`，`persistence_scope=none`，`sync_scope=none`。既有 sidecar map、driver update、virtual `AgentPromptUpdated` event 与同 instance snapshot projection 可以保留，但不能把它们解释为 durable recovery、cross-session replication 或 cross-entry sync。
- AC-PROMPT-011 的 `applied` 必须来自 authority result；Viewer 不得从 ack arrival、`applyPromptAckLocally`、local snapshot、reconnect 或 reload 推导更大 scope。AC-PROMPT-004 Preview 与所有 blocked/rejected/stale 结果不得产生 profile/event mutation。
- Recovery-generation Prompt persistence、durable result receipts、跨 session replication 与 production-supervisor activation 不属于本合同；这些需要新的 system authority 与 acceptance set。实现入口仍分别由 protocol/runtime、Agent/runner、Viewer 与 QA leaf 消费，不能在本 system leaf 合并。

### 观测字段与控制完成边界

- live drive 只由一个可处理的 mailbox/runtime 事件触发；一次触发至多形成一次 runtime drive。空 mailbox 或零结果保持静默，不能合成事件、逻辑时间、完成态或进度；之后到达的有效触发仍可继续推进。
- `getState().logicalTime` 是当前 Web surface 的逻辑时间观测值；`eventSeq` 是独立的事件顺序观测值。两者都只能由收到的 runtime 消息推进，不能由客户端控制请求、空 mailbox 或重连计时器自行递增。
- `tick` 只作为 `logicalTime` 的兼容 alias 保留；它不是 browser polling、mailbox drive 或“每个 step 必有进展”的承诺。新自动化优先读取 `logicalTime` 与 `eventSeq`。
- `sendControl` 的 accepted/queued 只表示客户端已通过本地校验并已尝试发送。只有后续的 runtime snapshot/event 或 completion feedback 才能标记观察到的增量；无增量不得伪造 event。
- `seek_event` 与 `seek` 一样不是 live 控制动作。playback 可按 profile 暴露 seek；live 必须在发送前返回 unsupported-action，而非以断链、重连或成功替代该结果。
- `prompt_control`、`agent_chat`、目标重排和记忆纠正的本地发送、accepted/queued 同样不等于 runtime 已应用或已产生世界后果。Viewer 只能按 canonical snapshot/feedback 呈现 accepted/applied/rejected/blocked、原因、影响范围和可执行恢复入口；不得从客户端状态、重连或 `logicalTime`/`eventSeq` 推导这些结果。玩家因果与恢复合同见 [`间接控制 agency 合同`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md)，Prompt 结果语义见 [`Agent 对话与 Prompt 控制`](../../product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md) 与 [`Viewer 手册`](viewer-manual.manual.md)。
- live 的 `seek` / `seek_event` 不支持不是玩家纠正、重排或恢复机制。纠正、重排和续玩只能走权威的 Prompt/Agent feedback、`reconnect_sync` / resume anchor 或明确的 reprioritize flow，不能通过本地回退逻辑时间、事件序号或重放 Viewer snapshot 实现。记忆驱动行动时，Viewer 仅呈现脱敏的摘要、来源、当前用途、staleness 和 correction outcome；不得暴露私有 prompt、内部 trace，也不得自造记忆真值。

## 验证与边界

- 不得把 live seek 的发送前拒绝或 legacy handler 的记录并忽略误报为断链、发送成功或世界回退；若未来引入 profile-specific dispatch 结果，必须同时实现并测试所有保留的 live 请求路径。
- 断链、被 gameplay gate 阻断、无可观察进展和不支持的 profile action 是不同状态；实现与 automation 不得将其中任一状态折叠为 fabricated event 或连接成功。
- 定向验证必须覆盖“空 mailbox 不输出、不推进观测值”以及“其后的有效触发仍能继续 drive”；Viewer 的 event drive 不拥有 node/consensus tick。
- Viewer UI、timeline、automation 与 `window.__AW_TEST__` 的 `getState`、`describeControls`、`fillControlExample`、`sendControl` 必须对齐 `controlProfile` 与支持集合。
- 受影响实现改动应覆盖协议 round-trip、live/playback handler、发送路由和 Web test API；可见 UI 改动按 `testing-manual.md` S6 提供 browser/console/语义证据。本轮仅迁移文档，不产生 browser 证据。

## 里程碑

- M1（已完成）：协议、握手与 server/live handler 按 profile 拆分。
- M2（已完成）：Viewer 路由、Web test API、automation 与 timeline 对齐 live 无 seek。
- M3（已完成）：已建立稳定 authority、切换入口并退役已吸收源三件套。

## 风险

- 若客户端忽略 profile，可能把不支持的 seek 误报为断链或发送成功；当前文档必须保留 Web 发送前拒绝与 legacy handler 忽略的区别。
- `seek` 保留为非 live 兼容枚举，不能被误解为 live 能力。
- 文档迁移不验证 UI 行为；未来任何可见控制面改动仍需单独 S6 取证。

## 追溯

当前实现入口包括 `crates/oasis7_proto/src/viewer.rs`、`crates/oasis7/src/viewer/{protocol.rs,server.rs,live_controls.rs,runtime_live.rs}`，以及 `crates/oasis7_viewer/software_safe_src/{legacy_core.js,legacy_core_control_gate.test.js}`；生成的浏览器入口为 `crates/oasis7_viewer/viewer.js`。操作入口见 `viewer-manual.manual.md`。
