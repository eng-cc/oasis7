# Viewer 控制面：回放与 Live 分离设计

> 对应需求: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`
> 对应执行台账: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.project.md`

## 结构

1. **协议与握手层**：`ViewerControlProfile`、`PlaybackControl`、`LiveControl` 与 `HelloAck.control_profile` 让客户端知道服务端的动作集合。
2. **服务端与路由层**：playback 和 live 各执行所属控制；legacy `Control` 只桥接兼容请求。发送前按 profile 校验，并返回 `Sent`、`UnsupportedForProfile` 或 `ClientChannelSendFailed` 等可诊断结果。
3. **表现层**：timeline、egui、Web test API 与 automation 共用 profile 判定；live 不显示可提交 seek，不能以“重连后再试”掩盖语义不支持。
4. **兼容层**：profile 未知时可临时显示 legacy 动作集合，但实际发送仍二次校验，避免竞态导致 live seek。

## 不变量

- live 的世界推进单调，不提供回退或跳时控制。
- `seek` 枚举可以为非 live 兼容保留，但不得泄漏为 live 玩家或自动化动作。
- UI/API 的结构化状态与真实 dispatch 必须一致；不可用不等于 channel failure。

## 代码与验证接点

- 协议/服务：`crates/oasis7_proto/src/viewer.rs`、`crates/oasis7/src/viewer/{protocol.rs,server.rs,live_split_part2.rs,mod.rs}`。
- 表现/自动化：`crates/oasis7_viewer/src/{timeline_controls.rs,viewer_automation.rs,web_test_api.rs,headless.rs}` 及相关面板模块。
- 本设计不改变世界规则、P2P/共识回退策略、视觉方向或测试发布判断。
