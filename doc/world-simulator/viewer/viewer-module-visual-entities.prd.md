# oasis7 Simulator：WASM 模块驱动新事物的通用可视实体机制

- 对应设计文档: `doc/world-simulator/viewer/viewer-module-visual-entities.design.md`
- 历史模块渲染实施与验证记录：GitHub task issue evidence。

审计轮次: 5

## 1. Executive Summary

- 为“模块驱动的新生事物”提供统一承载，避免每新增一种模块能力就新增一套专用 simulator 实体与 viewer 渲染分支。
- 将可视化语义下沉为 **runtime/simulator 可发布的数据实体**：模块只需要上报“我想在世界中可视化什么”，viewer 统一渲染。
- 与“默认电力设施语义下沉为场景配置”保持一致：基础供能由 runtime 预置模块承担，模拟器只维护通用世界状态与可视化载体。

## 2. User Experience & Functionality

### In Scope
- 在 simulator 增加通用 `ModuleVisualEntity` 数据结构与初始化/场景配置接入。
- 在 action/event 层提供模块可调用的通用更新接口：`UpsertModuleVisualEntity` / `RemoveModuleVisualEntity`。
- 在回放链路保证该类事件可重放，保持快照与事件一致。
- viewer 侧新增统一渲染路径：从 `snapshot.model.module_visual_entities` 与增量事件中构建/更新 marker。
- viewer 详情面板支持展示模块可视实体的基础信息（module/kind/anchor/label）。
- 事件列表对象联动支持模块可视实体（点击事件可定位对应 marker）。
- 右侧信息区支持滚动浏览，保证长详情/长事件列表可访问。

### Out of Scope
- 不定义具体 WASM ABI 的细节字段（本期仅定义 simulator/viewer 内部数据形态）。
- 不做模块可视实体的复杂物理交互（碰撞、力学、资源账本影响）。
- 不引入独立的“模块可视实体治理权限系统”。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### `ModuleVisualEntity`
- `entity_id: String`：实体唯一 ID（世界内唯一）。
- `module_id: String`：来源模块 ID（例如 `m1.power.radiation_harvest`）。
- `kind: String`：实体类别（如 `radiation_harvester`、`relay`、`beacon`）。
- `label: Option<String>`：可选显示名。
- `anchor: ModuleVisualAnchor`：锚点。

### `ModuleVisualAnchor`
- `Agent { agent_id }`：挂载在 Agent。
- `Location { location_id }`：挂载在 Location。
- `Absolute { pos }`：世界绝对坐标。

### 世界模型与事件
- `WorldModel.module_visual_entities: BTreeMap<String, ModuleVisualEntity>`（`serde default`）。
- `Action` 增加：
  - `UpsertModuleVisualEntity { entity }`
  - `RemoveModuleVisualEntity { entity_id }`
- `WorldEventKind` 增加：
  - `ModuleVisualEntityUpserted { entity }`
  - `ModuleVisualEntityRemoved { entity_id }`

### 初始化/场景
- `WorldInitConfig.module_visual_entities` 与 `WorldScenarioSpec.module_visual_entities`。
- 初始化时校验：
  - `entity_id` 非空且不冲突；
  - 锚点存在（Agent/Location）或坐标在空间边界内（Absolute）。

### Viewer 渲染约定（V1）
- 模块可视实体采用统一 marker 渲染路径（单一 mesh/material 族）。
- 渲染位置由 `anchor` 解析；`label` 为空时回退为 `kind:entity_id`。
- 通过增量事件做 upsert/remove，避免全量重建。

### Viewer 交互约定（V1.1）
- `ModuleVisualEntityUpserted/Removed` 事件可映射为对象联动目标（`SelectionKind::Asset` 复用路径）。
- 对象实体查找顺序：先真实资产 `asset_entities`，再模块可视实体 `module_visual_entities`。
- 右侧内容区域启用滚动条，滚轮仅在右侧面板命中时生效，避免干扰 3D 视图缩放。

## 5. Risks & Roadmap

- **M1**：设计文档与GitHub Issue/Project task truth。
- **M2**：simulator 数据结构、action/event、初始化与回放链路打通。
- **M3**：viewer 通用渲染 + 基础详情展示。
- **M4**：测试回归、文档状态回写、任务日志与提交。
- **M5**：补齐事件联动与右侧滚动交互。

### Technical Risks

- **ID 冲突**：模块侧若未约束命名空间，可能与其他实体产生重名冲突。
- **锚点失效**：锚点对象被删除或场景不含对应对象时，需要明确拒绝策略。
- **信息噪声**：模块实体过多会挤占视野，后续可能需要分层过滤。
- **语义漂移**：若模块把业务状态直接塞入标签文本，可能导致可视化不可检索；后续可考虑结构化扩展字段。

## 6. Validation & Decision Record
- 追溯: 历史模块渲染实施与验证记录由 GitHub task issue evidence 维护。
