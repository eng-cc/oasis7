# 玩家可读的世界舞台产品设计

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/player-readable-world-stage.prd.md`](player-readable-world-stage.prd.md)
- 产品迁移追踪：[`doc/product/agents-world-simulation/player-readable-world-stage.project.md`](player-readable-world-stage.project.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`

本文定义跨 surface 的玩家阅读顺序与视觉语义，不冻结历史 Bevy/EGUI 控件、2D/3D 模式、renderer 算法或布局参数。当前 surface 和视觉实现由 Viewer 手册、视觉规范及对应代码/测试 authority 定义。

## 1. 阅读顺序

1. 当前目的、关键 blocker 与下一决策。
2. 与当前目的相关的 Agent、地点、路线或关系。
3. 已接受行动的主要结果、成本与恢复路径。
4. 世界背景与次级支持信息。
5. 按需展开的诊断和原始数据。

surface 可以采用地图、舞台、列表、详情或响应式组合，但不能让诊断、环境装饰或信息密度压过玩家目的与因果。

## 2. 对象辨认与聚焦

- 当前相关对象及选中、聚焦或不可用状态必须可辨认，且不能只依赖颜色或运动。
- 没有相关 Agent、地点或路线时，明确显示为空、不可用或等待权威数据；不得用任意 fallback 对象制造虚假目标。
- 选中或视觉强调只改变呈现，不自动授予控制权、交互能力或世界事实。
- surface 可以简化背景或父级上下文来降低噪声，但仍要让玩家理解当前对象与世界、目标或关系的联系；被隐藏的环境细节不能被误解为世界事实已经消失。

## 3. 空间与比例诚实

- 权威位置、派生关系位置和纯呈现布局必须可区分。
- 当 surface 放大 marker、halo、标签或 overview 表示以保证可读性时，文本或权威数值优先；呈现尺寸不得冒充物理几何。
- 关系线、箭头、流向或其他视觉辅助只解释专业 authority 已支持的关系或效果，不创造路线、因果、所有权或动作入口。
- 高密度时优先保留当前目标、选中对象和关键 blocker；次级标签可以渐进隐藏，但不得通过无差别缩小使首读对象不可辨认。
- overview 与 detail 可以使用不同密度、尺度和标注，但切换后仍保持目的、选中对象、关键 blocker 和下一步连续；这项原则不要求存在专用地图模式、自动缩放或历史 Viewer 控件。

## 4. 支持文本与诊断

- 玩家目标、blocker、回执和恢复路径应能以可访问文本或等价语义读取。
- 支持信息可以被选择、复制或导出，但只有当前 surface authority 明确支持时才展示对应动作。
- 原始 DTO、renderer 状态、性能指标与调试记录默认次级，不得成为产品成功的代签证据。
- 选中对象详情只解释当前 surface authority 已支持的上下文和因果；原始 LLM 输入输出、模型、token、延迟或 runtime 诊断不自动成为玩家层详情。

## 5. 响应式与可访问性

- CJK、窄屏和低高度下仍应保持首读对象、主要状态与下一步可辨认。
- 颜色、图标、大小或动效不能成为唯一状态信号。
- 布局可改变，但阅读顺序、权威来源与无虚假交互的边界保持稳定。

## 6. 非承诺

- 不承诺 `Locate Agent`、尺寸检查面板、复制面板、2D/3D 切换或 overview-map 控件存在。
- 不定义 centimetre 字段、比例/clamp、marker 几何、箭头、LOD 阈值、标签容量或截图基线。
- 不把历史完成状态、EGUI/Bevy 组件、测试命令或截图提升为当前产品能力。
