# Viewer Chat Web IME EGUI 输入桥接设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.prd.md`
- 历史实施与 task 状态：GitHub task issue evidence。

## 1. 设计定位
定义 wasm Viewer 中 DOM IME 组合事件到 `egui::Event` 的桥接路径，通过隐藏 HTML input 与桥接资源补齐 `bevy_egui` 在 Web 中文输入上的能力缺口。

## 2. 设计结构
- DOM 捕获层：隐藏 input 捕获 `composition*`、`input` 与必要键盘事件。
- bridge 事件层：把 DOM 事件转成桥接资源与内部事件通道。
- EGUI 注入层：将桥接结果写入 Primary EGUI Context。
- 聚焦驱动层：在文本编辑激活时 focus bridge，非编辑态自动 blur。

## 3. 关键接口 / 入口
- `wasm_egui_input_bridge.rs`
- `compositionstart/update/end`
- `egui::Event`
- `EguiInputEvent`
- `bevy_egui` 主上下文

## 4. 约束与边界
- 不修改 Viewer 协议字段和业务 Chat 流程。
- 事件映射需考虑浏览器时序差异并做去重控制。
- bridge 聚焦策略不能破坏非文本快捷键行为。
- 本轮只修 Web IME 输入桥接，不改 native 输入法行为。

## 5. 设计演进计划
- 先搭建 DOM -> bridge -> EGUI 的最小事件通路。
- 再打磨聚焦策略与回修现场反馈问题。
- 最后以 Web 闭环和回归测试固定 IME 桥接基线。
