# 玩家可读的世界舞台迁移追踪

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/player-readable-world-stage.prd.md`](player-readable-world-stage.prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/player-readable-world-stage.design.md`](player-readable-world-stage.design.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 追踪范围：产品文档语义迁移
- Owner role：`repository_health_engineer`

本文只记录产品语义归位和源文件删除条件，不维护执行计划、checkbox、PR、CI、发布或任务状态。

## 迁移映射

| 已吸收源专题 | 归位语义 | 未提升为产品承诺 |
| --- | --- | --- |
| `viewer-agent-quick-locate` 三件套 | 首读可辨认相关 Agent、地点或路线；空状态必须诚实 | Locate 控件、fallback 选择、相机动作、面板位置 |
| `viewer-agent-size-inspection` 三件套 | 表现比例和派生呈现不得冒充权威世界事实；该语义已由产品 PRD 承载 | 尺寸检查面板、厘米字段、比例/clamp 与通用尺度保证 |
| `viewer-copyable-text` 三件套 | 玩家目标、blocker、回执等支持文本保持可访问；诊断默认次级 | 复制面板、剪贴板支持、默认可见诊断清单 |
| `viewer-2d-visual-polish` 三件套 | Agent、地点、路线与支持关系保持可读；视觉辅助不生成权威或交互 | 2D/3D 模式、地图控件、marker/箭头/LOD、资产与截图结论 |
| `viewer-frag-default-rendering` 三件套 | 世界背景支持首读对象，呈现细节不构成世界权威 | Location 隐藏策略、渲染开关、环境变量与历史默认模式 |
| `viewer-frag-scale-selection-stability` 三件套 | 表现尺度、选中和强调不得冒充物理事实或交互资格 | scale/clamp/hash、几何、hit testing 与 Bevy/EGUI 实现 |
| `viewer-fragment-element-rendering` 三件套 | 环境与材质提供世界上下文，但不暗示交互或玩家结果 | block 渲染、材质管线、手动开关、性能与测试实现 |
| `viewer-generic-focus-targets` 三件套 | 相关对象的聚焦、选中与不可用状态可辨认且不制造 fallback 目标 | FocusTarget grammar、kind mapping、Test API 与自动化合同 |
| `viewer-overview-map-zoom` 三件套 | overview/detail 变化仍保留目的、选中对象、blocker 与下一步 | 2D 地图、自动缩放、阈值、orbit 默认值与 3D 行为 |
| `viewer-selection-details` 三件套 | 当前对象与受支持因果上下文可读，诊断仍属次级 | 详情面板、LLM I/O、布局滚动、hover/pressed 与像素坐标 |

## 删除收据

- 本轮新增删除源文件：18；本专题累计吸收并删除源文件：30。
- 活跃索引改指产品 PRD/design 与当前 Viewer/视觉专业权威。
- 当前 surface、world-unit truth、presentation mapping、组件行为、视觉规范与验证继续由 Viewer/视觉专业 authority 承载。
- 历史 EGUI/Bevy surface、字段、算法、布局、测试和完成状态只通过 Git history 与 GitHub task evidence 追溯。

## 完成条件

本收据仅在配对 PRD/design 可由模块产品树进入、上述十组源文件整组删除、活跃引用修复、旧专用控件未被描述成当前能力时有效。`viewer-module-visual-entities` 三件套因仍承载活跃 runtime/action/event/replay 专业合同而明确保留，不计入本收据。合并、review、CI、发布和任务完成事实不由本文件证明。

## 任务拆解

不适用。本文件不维护执行计划、checkbox、owner 队列或当前任务进度；相关真值只进入 GitHub task issue evidence。

## 依赖

- [`doc/product/README.md`](../README.md) 的产品/专业边界和迁移删除治理。
- [`doc/world-simulator/viewer/viewer-manual.manual.md`](../../world-simulator/viewer/viewer-manual.manual.md) 的当前 surface 与退役边界。
- [`doc/world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md`](../../world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md) 的当前视觉专业权威。

## 状态

- 文档生命周期：`active`。
- 迁移收据：`finalized`，只表示上述产品语义和专业边界已归位。
