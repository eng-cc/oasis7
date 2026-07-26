# Viewer 控制面：回放与 Live 分离设计

> 对应需求: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`
> 对应执行台账: `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.project.md`

## 结构

1. **协议与握手层**：`ViewerControlProfile`、`PlaybackControl`、`LiveControl` 与 `HelloAck.control_profile` 让客户端知道服务端的动作集合。
2. **服务端与路由层**：playback 和 live 各执行所属控制；legacy `Control` 只桥接兼容请求。现有 legacy live handler 收到 seek 时记录并忽略，保持世界不回退；这不是 profile-specific 结构化响应。
3. **表现层**：timeline、egui、Web test API 与 automation 不把 live seek 暴露为可提交动作；当前 Web 控制入口对 seek 在发送前给出通用 unsupported-action 反馈，不能以“重连后再试”掩盖语义不支持。
4. **兼容层**：profile 未知时可临时显示 legacy 动作集合；若旧调用方仍发送 seek，live handler 的 log-and-ignore 行为必须被视为兼容性边界，而不是成功、断链或回退。
5. **观测层**：`logicalTime` 与 `eventSeq` 分别表达已观察的逻辑时间和事件顺序；`tick` 仅为前者的兼容 alias。控制请求、重连计时器与空 mailbox 不得合成任一观测增量。

## 不变量

- live 的世界推进单调，不提供回退或跳时控制。
- `seek` 枚举可以为非 live 兼容保留，但不得泄漏为 live 玩家或自动化动作。
- Web 发送前拒绝与 legacy handler 的记录并忽略必须如实区分；两者都不等于 channel failure、发送成功或世界回退。
- queued/accepted 是本地发送阶段，不能替代 runtime 事件、snapshot 或 completion feedback；空结果必须保持空结果。

## 代码与验证接点

- 协议/服务：`crates/oasis7_proto/src/viewer.rs`、`crates/oasis7/src/viewer/{protocol.rs,server.rs,live_controls.rs,runtime_live.rs}`。
- 表现/自动化：`crates/oasis7_viewer/software_safe_src/{legacy_core.js,legacy_core_control_gate.test.js}`；生成浏览器入口为 `crates/oasis7_viewer/viewer.js`。
- 本设计不改变世界规则、P2P/共识回退策略、视觉方向或测试发布判断。
