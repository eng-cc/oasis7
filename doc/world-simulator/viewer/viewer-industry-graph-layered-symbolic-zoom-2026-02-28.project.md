# Viewer 产业链图谱化与分层符号化（项目管理）

- 对应设计文档: `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）

### T0 文档基线
- [x] T0.1 设计文档：`viewer-industry-graph-layered-symbolic-zoom-2026-02-28.md`
- [x] T0.2 项目管理文档：本文件

### T1 统一中间层
- [x] T1.1 新建 `IndustryGraphViewModel`（节点/边/层级/状态/根因链/zoom 过滤）
- [x] T1.2 增加模型构建单测

### T2 文本聚合改造
- [x] T2.1 `ui_text_industrial.rs` 改为消费图谱
- [x] T2.2 `ui_text_economy.rs` 改为消费图谱

### T3 产业链主图层 + 分层符号
- [x] T3.1 `world_overlay.rs` 接入图谱边渲染（吞吐线宽、流型颜色）
- [x] T3.2 节点符号系统（R1~R5 形态、阶段外环、瓶颈/拥塞/告警角标）

### T4 运营导航根因链
- [x] T4.1 `ui_text_ops_navigation.rs` 输出根因链图结构文本
- [x] T4.2 输出可跳转目标标识（target）

### T5 语义缩放
- [x] T5.1 `egui_right_panel.rs` 增加世界/区域/节点缩放控件
- [x] T5.2 文本与 overlay 按 zoom 分层展示

### T6 测试与收口
- [x] T6.1 test_tier_required：viewer 定向单测
- [x] T6.2 test_tier_full：`oasis7_viewer` 全量测试 + wasm check
- [x] T6.3 更新项目状态与 devlog

## 依赖
- doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.prd.md
- `crates/oasis7_viewer/src/ui_text_industrial.rs`
- `crates/oasis7_viewer/src/ui_text_economy.rs`
- `crates/oasis7_viewer/src/ui_text_ops_navigation.rs`
- `crates/oasis7_viewer/src/world_overlay.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/ui_locale_text.rs`
- `crates/oasis7/src/simulator/kernel/types.rs`
- `doc/world-runtime/prd.md`
- `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`

## 状态
- 当前阶段：已完成（T0~T6 全部收口）
- 下一阶段：若要深化交互，可在根因链 `target` 基础上接入直接点击定位动作
- 最近更新：完成图谱中间层、主图层分层符号、根因链与语义缩放，并通过 viewer 全量回归（2026-02-28）
