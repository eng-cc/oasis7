# Viewer Minimal System Demo 设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-minimal-system.prd.md`
- 历史 demo 实施与验证记录：GitHub task issue evidence。

## 1. 设计定位
定义最小化 Viewer Demo 数据生成方案：通过一个确定性 CLI 生成 `snapshot.json` + `journal.json`，让新用户可以低摩擦完成“生成数据 -> 启动 server -> 打开 UI”的闭环。

## 2. 设计结构
- CLI 入口层：`oasis7_viewer_demo` 提供场景选择和输出目录参数。
- 演示脚本层：按确定性策略选择 agent/location 并生成至少一个事件。
- 持久化输出层：产出 `snapshot.json` 与 `journal.json` 供 `oasis7_viewer_server` 消费。
- 文档引导层：README/quick start 给出最小端到端运行路径。

## 3. 关键接口 / 入口
- `oasis7_viewer_demo [scenario] [--out <dir>]`
- `snapshot.json`
- `journal.json`
- `oasis7_viewer_server` / `oasis7_viewer`

## 4. 约束与边界
- 这是离线 replay demo，不包含 live simulation server。
- 演示脚本只需保证低摩擦和有事件，不追求复杂玩法。
- 输出必须可重复生成，避免示例数据每次漂移。
- UI/Bevy 功能扩展不在本专题范围。

## 5. 设计演进计划
- 先实现 demo 数据生成 API。
- 再补 CLI 与持久化测试。
- 最后把最小运行步骤写进文档入口。
