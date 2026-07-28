# Viewer 控制面：回放与 Live 分离

> 本文是回放/Live 控制 profile 与 live 无 seek 语义的当前专业 authority。它收敛两个 2026-02 源三件套；历史变更仅从 Git 与 GitHub task evidence 追溯。

- 对应设计: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.design.md`
- 对应项目: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.project.md`

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

### 观测字段与控制完成边界

- live drive 只由一个可处理的 mailbox/runtime 事件触发；一次触发至多形成一次 runtime drive。空 mailbox 或零结果保持静默，不能合成事件、逻辑时间、完成态或进度；之后到达的有效触发仍可继续推进。
- `getState().logicalTime` 是当前 Web surface 的逻辑时间观测值；`eventSeq` 是独立的事件顺序观测值。两者都只能由收到的 runtime 消息推进，不能由客户端控制请求、空 mailbox 或重连计时器自行递增。
- `tick` 只作为 `logicalTime` 的兼容 alias 保留；它不是 browser polling、mailbox drive 或“每个 step 必有进展”的承诺。新自动化优先读取 `logicalTime` 与 `eventSeq`。
- `sendControl` 的 accepted/queued 只表示客户端已通过本地校验并已尝试发送。只有后续的 runtime snapshot/event 或 completion feedback 才能标记观察到的增量；无增量不得伪造 event。
- `seek_event` 与 `seek` 一样不是 live 控制动作。playback 可按 profile 暴露 seek；live 必须在发送前返回 unsupported-action，而非以断链、重连或成功替代该结果。
- `prompt_control`、`agent_chat`、目标重排和记忆纠正的本地发送、accepted/queued 同样不等于 runtime 已应用或已产生世界后果。Viewer 只能按 canonical snapshot/feedback 呈现 accepted/applied/rejected/blocked、原因、影响范围和可执行恢复入口；不得从客户端状态、重连或 `logicalTime`/`eventSeq` 推导这些结果。玩家因果与恢复合同见 [`间接控制 agency 合同`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md)，Prompt 结果语义见 [`Agent 对话与 Prompt 控制`](../../../product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md) 与 [`Viewer 手册`](viewer-manual.manual.md)。
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
