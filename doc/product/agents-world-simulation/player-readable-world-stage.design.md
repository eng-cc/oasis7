# 玩家可读的世界舞台产品设计

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/player-readable-world-stage.prd.md`](player-readable-world-stage.prd.md)
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

## 1.1 P0 冻结与 P1-A Agent Context Lite

### P0：稳定框架，不扩大常驻界面

P0 的设计收口是稳定阅读位置和层级，而不是固定占据空间的三栏。世界舞台继续承担
primary read；目的、关键 blocker、下一步、相关对象和 Action Receipt 构成相邻但可按需
展开的解释路径。后续设计不以重新安排 Shell、增加常驻栏位或直接动作入口作为前置条件。

Focused 的信任、来源时效、权限披露、回执唯一性和窄屏可读性修正可以继续进行，但必须
保持世界优先、间接控制和单一因果锚点不变。

### P1 有序组合

产品设计按 P1-A、P1-B、P1-C 依次展开：

1. **P1-A Agent Context Lite**：先让一个选中的 Agent 具备可读身份、状态、来源/时效、
   目标、下一步、阻塞、Activity、Intent、单一因果回执和权限边界。它依赖已有的 Agent
   身份/绑定与权威投影；选中或可见不等于控制。
2. **P1-B World semantic presentation**：在 P1-A 的对象、来源和首读语义稳定后，才加入
   有明确 authority 的世界对象、关系/路线和位置语义。关系必须保留种类、状态、来源与
   时效；本阶段不以视觉邻近或关系表达创造动作资格、控制权或新的 Agent 语义。
3. **P1-C Major World Event**：在事件身份、种类/严重度、来源、因果锚点、时效、生命周期
   与重放/重组边界稳定后，才加入重大事件的注意力语义。事件提示下一次观察或决策，但
   不成为 Action Receipt，也不创建第二条因果链。

P1-B 和 P1-C 的依赖是产品语义和专业证据均已准备；不以局部 surface、技术信号或设计
稿名称声称阶段已实现。每一阶段都保留上一阶段的世界优先、间接控制和单一因果锚点。
当前专业实现收据已覆盖 P1-B stage cues 与 P1-C crisis-only ambient projection；crisis 没有
权威空间位置时不绘制 marker/highlight。runtime/operator audience policy 是独立显式输入，
默认 Unknown 不披露，不能从登录、选择、控制或 Director 权限推导。

### P1-A：只从选中的 Agent 开始

Agent Context Lite 的设计对象是“当前选中的一个 Agent”，不是所有实体的统一详情模板。
它的阅读组合遵循以下顺序：

1. **辨认**：可读身份，以及有权威来源的所在/父级上下文和状态；从世界或目标上下文选中
   后，目标语义应在 0–1 次情境转换内可读。
2. **理解当前局势**：只有明确绑定并适用于该 Agent 的 Objective、推荐 Next Move、blocker
   与 Player Leverage/执行状态；global summary/primary intent 保持玩家层上下文，不改名
   为该 Agent 的私有状态。
3. **区分运行语义**：Activity 表示 Agent 正在做什么，Intent 表示明确匹配该 Agent 的权威
   接受或阻断；`local_pending` 仍是玩家未提交草稿，二者都不能代替世界结果。
4. **确认结果**：只引用现有 Player/Cinematic 因果锚点中的单一 Action Receipt。非因果反馈
   可以独立出现但不能补成回执、效果或成功；没有该锚点就明确保持无回执。
5. **继续或恢复**：只有在 capability 来源、权限、时效和恢复路径完整时，才表达下一受支持
   入口；否则说明不可用、待同步或应重新确认。选中、可见或视觉强调只帮助辨认，不授予控制。

账号绑定、认领和当前控制资格沿用[`Agent 对话与 Prompt 控制`](agent-conversation-and-prompt-control.prd.md)
与[`Agent 权限、资产与责任连续性`](agent-authority-ownership-and-accountability.prd.md)；本专题
不把 selection/visibility 变成控制 authority，也不把 global summary 变成 Agent 私有事实。

该顺序可以跨舞台、目标、列表、详情或响应式 surface 保持，但不规定控件、栏位、动画、
布局参数或具体手势。选中和视觉强调只帮助辨认，不会产生控制权、ownership、关系或动作
资格。

Location、Facility、Territory、Organization、Depot 和 Module 不得复用 Agent 的默认语义。
它们后续需要各自的身份、来源、权限、时效、因果和恢复合同；缺少合同时应保持为空或不可用，
而不是用邻近 Agent 的数据填充。

### P1-A 组合验收映射

| 设计检查 | 可观察承诺 | 产品验收 |
| --- | --- | --- |
| Agent identity/context | 选中的 Agent 与其相关状态可辨认，缺失时不制造默认目标。 | P1A-CTX-01 / RW-1 / RW-6 |
| Source and freshness | 当前确认、最近已知、冲突和未知保持不同语义，并提供适用的恢复方向。 | P1A-CTX-02 / RW-8 / RW-9 |
| Activity/Intent/Receipt | Agent 活动、明确匹配的权威意图、Player/Cinematic 单一因果锚点和非因果反馈不会合并；环境活动不能代签回执。 | P1A-CTX-03 / RW-3 / RW-4 |
| Capability boundary | 只有明确绑定、适用来源和权限的 Agent capability 才能进入玩家语义；选中/可见不等于可控，global summary 不会改变控制边界。 | P1A-CTX-04 / RW-2 / RW-6 |
| World-first readability | 解释层保持次级且可按需访问，首读仍是世界、目的、阻塞和下一步。 | P1A-CTX-05 / RW-1 / RW-4 / RW-5 |

### P1-A 非承诺

- 不定义 Agent 之外实体的详情结构或统一 Inspector schema。
- 不定义新的世界动作、hotbar、global inventory、经济/治理面板或数值平衡。
- 不把 global summary、global primary intent 或 `local_pending` 草稿改名为选中 Agent 的私有状态、
  已接受 Intent 或因果回执。
- 不把关系线、距离、同屏、环境事件或视觉强调当作关系、Trust、ownership 或 capability 来源。
- 不把 plan、rationale、ETA、memory、provider、prompt、auth 或 runtime 诊断当作默认玩家详情。
- 不在本切片设计 Major World Event 的 marker、highlight、toast 或第二个因果反馈面。
- 不定义 DTO、坐标、renderer 层级、布局参数、断点、颜色或具体可访问性控件；这些继续由
  对应专业 authority 负责。

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
