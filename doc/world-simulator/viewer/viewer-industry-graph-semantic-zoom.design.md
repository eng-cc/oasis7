# Viewer 产业链图谱与语义缩放设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-industry-graph-semantic-zoom.prd.md`
- 已完成的实施追溯见 GitHub task issue evidence comments 与 `.pm/github-project-sync/task-archive.jsonl`。

## 1. 设计定位
定义工业/经营/运营导航统一图谱化方案：通过 `IndustryGraphViewModel` 把文本摘要、Overlay 主图层和语义缩放收敛到同一中间层，形成可跳转、可分层、可聚焦的产业链视图。

## 2. 设计结构
- 中间层模型：`IndustryGraphViewModel` 统一节点、边、层级、状态和根因链。
- 文本消费层：工业/经营/运营导航文本都改为从图谱模型生成。
- Overlay 主图层：使用节点符号、吞吐线宽和流型颜色表达主干流与告警。
- 语义缩放层：World/Region/Node 三档缩放联动文本细节级与 Overlay 可见集。

## 3. 关键接口 / 入口
- `industry_graph_view_model.rs`
- `ui_text_industrial.rs` / `ui_text_economy.rs` / `ui_text_ops_navigation.rs`
- `world_overlay.rs`
- `IndustrySemanticZoomState`

## 4. 约束与边界
- 不改 simulator 协议与事件系统，只基于现有 snapshot/event 推导图谱。
- 节点符号系统复用现有 mesh/material 组合，不引入新的美术导入流程。
- 根因链输出先提供可跳转目标标识，不在本轮重写事件联动体系。
- 缩放联动必须控制 Overlay 负载，避免节点/边全部常显。

## 5. 设计演进计划
- 先建立图谱中间层与模型测试。
- 再迁移文本摘要和 Overlay 图层。
- 最后接语义缩放控件与根因链跳转入口。
