# Viewer Chat Web 锁重入修复设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.prd.md`
- 历史实施与 Web 闭环证据：GitHub task issue evidence。

## 1. 设计定位
定义 Web Viewer 在 Chat 面板中的锁重入与回车发送闭环修复方案，消除 `parking_lot` panic，并保证 wasm 模式下“输入消息 -> 发送给 Agent”链路稳定可用。

## 2. 设计结构
- 重入规避层：修复 Chat 面板内 `egui::Context` 的重入锁路径。
- wasm 调试兼容层：约束 `epaint` 在 wasm dev 下的 debug 行为，避免 timed-lock panic。
- Enter 判定层：回车发送改用输入活跃态，兼容 wasm IME bridge 场景。
- 闭环验证层：通过 Web 运行、消息发送和状态回显验证修复效果。

## 3. 关键接口 / 入口
- `egui::Context`
- `AgentChatDraftState`
- `egui_right_panel_chat.rs`
- `wasm_egui_input_bridge.rs`
- `Cargo.toml` profile 针对 `epaint`

## 4. 约束与边界
- 不改 `AgentChat` 协议和 Agent 决策内核。
- profile 调整应尽量局部化到 `epaint` 相关配置。
- Enter 判定修复不能回归 `Shift+Enter` 和 IME 输入边界。
- 本轮不改 native 端交互设计。

## 5. 设计演进计划
- 先定位重入锁和 wasm panic 触发点。
- 再修复输入活跃态判定与 profile 兼容设置。
- 最后通过 Web 闭环取证确认 Chat 可稳定发送。
