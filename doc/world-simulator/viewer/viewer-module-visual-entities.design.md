# Viewer 模块可视实体通用机制设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-module-visual-entities.prd.md`
- 历史模块渲染实施与验证记录：GitHub task issue evidence。

## 1. 设计定位
定义“模块驱动的新事物”统一可视实体机制：让 runtime/simulator 只发布通用 `ModuleVisualEntity` 数据，viewer 负责统一渲染与详情呈现，避免每种模块能力都扩一套专用实体分支。

## 2. 设计结构
- 数据模型层：在 simulator 中引入通用 `ModuleVisualEntity` 与初始化/场景接入。
- 事件更新层：通过 `UpsertModuleVisualEntity` / `RemoveModuleVisualEntity` 维持快照与增量一致。
- Viewer 渲染层：从 `snapshot.model.module_visual_entities` 和事件流构建 marker。
- 详情联动层：右侧详情、事件对象联动和滚动浏览都支持模块可视实体。

## 3. 关键接口 / 入口
- `ModuleVisualEntity`
- `UpsertModuleVisualEntity` / `RemoveModuleVisualEntity`
- `snapshot.model.module_visual_entities`
- viewer 详情与事件联动入口

## 4. 约束与边界
- 本期只定义 simulator/viewer 内部数据形态，不展开 WASM ABI 细节。
- 不做复杂物理交互或独立治理权限系统。
- 新增模块能力优先发布通用实体，而不是继续堆专用渲染分支。
- 长详情和长事件列表必须可滚动访问，不能因实体变多而失去可读性。

## 5. 设计演进计划
- 先落通用实体模型和事件接口。
- 再接 viewer 渲染与详情联动。
- 最后通过回放一致性和 UI 测试收口模块可视机制。
