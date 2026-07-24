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

## 验证与边界

- 不得把 live seek 的发送前拒绝或 legacy handler 的记录并忽略误报为断链、发送成功或世界回退；若未来引入 profile-specific dispatch 结果，必须同时实现并测试所有保留的 live 请求路径。
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
