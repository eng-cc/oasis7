# Viewer 产业链图谱化与语义缩放

- 对应设计文档: `doc/world-simulator/viewer/viewer-industry-graph-semantic-zoom.design.md`
- 已完成的实施追溯见 GitHub task issue evidence comments 与 `.pm/github-project-sync/task-archive.jsonl`。

审计轮次: 5

## 1. Executive Summary
- 将工业/经营/运营导航从“分散文本聚合”升级为统一 `IndustryGraphViewModel` 中间层。
- 在 3D 覆盖层落地“产业链主图层”：节点（工厂/配方/产品/物流站）、边（物料/电力/数据流）、线宽按吞吐、颜色按流类型。
- 依据 P3 分层档案语义实现“分层符号系统”：R1~R5 形态差异、阶段外环（bootstrap/scale/governance）、瓶颈/拥塞/告警角标。
- 将“运营导航”从列表改为根因链图（拒绝 -> 资源短缺 -> 路由拥堵 -> 产线停摆候选），并输出可跳转目标标识。
- 在右侧面板加入语义缩放（世界/区域/节点），实现远景看热区主干流、近景看配方与库存状态。

## 2. User Experience & Functionality

### In Scope
- 新增 `IndustryGraphViewModel` 及其节点/边/层级/状态数据结构。
- 改造 `ui_text_industrial.rs`、`ui_text_economy.rs`、`ui_text_ops_navigation.rs`，统一改为消费图谱模型。
- 改造 `world_overlay.rs`，使用图谱边渲染主干流，并增加节点符号渲染。
- 改造 `egui_right_panel.rs`，新增语义缩放控件并驱动文本与主图层细节级别。
- 新增/更新单元测试，覆盖模型构建、文本摘要、overlay 流段聚合与语义缩放行为。

### Out of Scope
- 不改动 simulator 协议（仍基于 `WorldSnapshot + WorldEvent` 推导图谱）。
- 不引入新美术资产导入流程（用现有 mesh/material 组合表达符号语义）。
- 不重写事件联动系统（本次提供“可跳转目标标识”，保留后续交互深化空间）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 新中间层
- 模块：`crates/oasis7_viewer/src/industry_graph_view_model.rs`
- 关键结构：
  - `IndustryGraphViewModel`
  - `IndustryGraphNode`（kind/tier/stage/status/position/chunk）
  - `IndustryGraphEdge`（flow_kind/throughput/loss/events）
  - `IndustrySemanticZoomLevel`（World/Region/Node）
  - `IndustrySemanticZoomState`（Bevy Resource）

### 2) 数据来源
- `WorldSnapshot.model.module_visual_entities`：节点语义与锚点。
- `WorldSnapshot.model.{agents,locations,assets}`：库存与位置。
- `WorldEventKind::{ResourceTransferred,PowerTransferred,CompoundRefined,ActionRejected,ModuleVisualEntityUpserted,...}`：流量、瓶颈、告警、拥塞信号。
- P3 文档语义映射：`R1~R5` 与 `bootstrap/scale/governance` 通过 id/tag 规则推断。

### 3) 视图层接口
- 文本侧：新增 `*_with_zoom(...)`，并保留旧函数作为默认缩放兼容入口。
- Overlay：按 zoom 过滤节点与边，远景聚焦热区/主干边，近景显示节点符号与库存态。
- Ops 导航：输出根因链条与 `target=` 标识（location/agent/module）。

## 5. Risks & Roadmap
- T0：设计文档 + 项目文档。
- T1：`IndustryGraphViewModel` 与单测。
- T2：工业/经营文本改造（统一模型输入）。
- T3：主图层与分层符号系统落地（world overlay）。
- T4：运营导航根因链图改造。
- T5：语义缩放接入右侧面板并联动文本/overlay。
- T6：测试回归、文档状态更新、devlog 收口。

### Technical Risks
- 事件语义不完整：部分根因链需基于代理规则推断，可能与真实业务链存在偏差。
- Overlay 负载上升：节点符号 + 边渲染增多，需依赖 zoom 和阈值裁剪。
- 文本兼容风险：旧文案字段迁移到图谱后可能影响既有断言，需要同步更新测试。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
