# 玩家可读的世界舞台

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/player-readable-world-stage.design.md`](player-readable-world-stage.design.md)
- 产品迁移追踪：[`doc/product/agents-world-simulation/player-readable-world-stage.project.md`](player-readable-world-stage.project.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`语义定位`](../../world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md)、[`Fragment LOD`](../../world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.prd.md)、[`玩家因果优先的渲染闭环`](../../world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md)

本文是长期产品分册，定义玩家观察 Agent 与世界模拟时的首读层级、空间关系、可归因因果与诊断边界。它不指定 Viewer 组件、DTO、派生算法、LOD 阈值、渲染管线、视觉资产或当前发布结论。

## 1. 目标

玩家进入正式世界表面后，应先读懂当前目标、相关 Agent 与地点或路线、关键阻塞、可采取的下一步，以及已接受行动造成的可理解世界反馈。环境、地形与世界活动提供必要背景，但不能压过玩家行动和因果。

没有已接受的玩家行动或可归因的权威世界变化时，产品必须明确这一事实；世界仍在活动、画面信息丰富、renderer 正常或调试数据可见，都不能被表达成玩家已经取得进展。

## 2. 首读层级与下一步

- 首读层优先呈现当前目的、相关行动者或路线、关键 blocker 和下一决策，而不是默认展示技术状态、原始数据或环境计数。
- 玩家能够把结果与当前目标联系起来，并找到继续、纠正、中断或恢复的下一步。
- 世界背景需要支持理解场景和关系，但不得以信息密度或视觉权重掩盖 Agent、目标、阻塞与玩家可施加的影响。
- 产品层只约束玩家能够读懂什么；具体布局、文案结构、交互控件、渲染层次与可访问性实现继续由 Viewer 与视觉交互专业权威拥有。
- 玩家能够辨认当前相关对象及其选中或聚焦状态；没有相关 Agent、地点或路线时，surface 必须诚实表达为空或不可用，不能通过默认选中制造虚假目标。

## 3. 空间关系与事实来源

- 玩家需要读懂当前相关 Agent、地点、路线、目标或 blocker 之间的关系，但产品不承诺所有对象都有精确坐标。
- 专业表面可以为稳定表达关系使用派生或抽象位置，但必须区分权威世界位置与仅用于呈现的关系位置；后者不得被包装成精确坐标、权威资源地点或可执行世界事实。
- 派生呈现不会产生新的控制权、目标权或世界规则。玩家动作仍通过受控入口提交，并由同一权威运行时接受、拒绝或产生替代结果。
- 地形、材料或环境结构可以作为世界上下文，但其详细度不得自动赋予 hover、selection、采集、建造或其他交互能力。
- 地图、关系线、方向标记、halo 或其他视觉辅助只用于解释已受支持的关系与效果，不会生成控制权、所有权、因果事实、精确位置或可执行能力。

## 4. 玩家因果与诊断边界

- 只有已接受的玩家意图及其可归因世界后果，才能被表达为玩家影响；环境变化、Agent 自主活动或渲染更新不能代签。
- 结果说明至少保留主要因果、成本或进展、阻塞，以及下一决策或恢复路径；详细规则、字段、数值与状态机回到 gameplay、runtime、Agent 和 Viewer 专业权威。
- renderer、runtime source、原始 DTO、性能状态与测试控制等诊断信息可以按需访问，但默认处于次级层，不得取代玩家目的、行动与反馈。
- 玩家当前目标、blocker 与行动回执应能以可访问的文本或等价方式被读取；这不承诺专用复制面板、剪贴板功能或默认暴露原始诊断文本。
- mock、fallback、截图或仅本地可见的调试表面不能单独证明正式玩家表面已经满足本产品承诺。

## 5. 组合验收

- RW-1：代表性正式玩家表面中，玩家无需阅读诊断信息即可识别世界上下文、当前目标、相关行动者或路线、关键 blocker 和下一决策。
- RW-2：用于说明决策的空间关系可读；任何派生或抽象位置都不会被表达为其并不具备的权威精度、资源事实或交互能力。
- RW-3：受支持意图被接受并产生世界结果后，玩家能够区分自己的可归因后果与环境或 Agent 自主活动，并找到继续、纠正、中断或恢复路径。
- RW-4：玩家目的、行动与反馈保持首要；诊断信息可按需访问，但不会成为默认首屏层级或整体成功的代签证据。
- RW-5：代表性端到端场景可追踪到 gameplay、runtime、Agent、Viewer 与 QA 的当前证据；单张截图、mock、fallback 或局部渲染通过不能单独成立产品结论。

### 5.1 验收权威与证据边界

| 产品承诺 | 专业 owner | 专业域权威 | 证据边界 | 测试层级 |
| --- | --- | --- | --- | --- |
| 首读层级与空间关系来源诚实 | viewer_engineer / game_visual_interaction_designer / qa_engineer | `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning-2026-05-26.prd.md`; `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod-2026-05-27.prd.md`; `doc/world-simulator/viewer/viewer-pixel-world-commercial-rendering-loop-2026-05-28.prd.md` | 正式玩家表面中的目标、关系、阻塞与下一步可读性，以及权威位置和派生呈现的区分；不复制 DTO、阈值或 renderer 合同 | test_tier_required |
| 可归因玩家因果与恢复路径 | gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 以已接受意图、权威世界结果、主要因果和下一决策或恢复路径组成端到端证据；环境活动不能代签玩家影响 | test_tier_full |

## 6. 范围

覆盖正式玩家 surface 的首读层级、相关对象与关系辨认、空间与比例诚实、可归因因果、支持文本和诊断降级。不覆盖具体 Viewer 控件、renderer、视觉资产或专业实现。

## 7. 接口 / 数据

产品层只定义 `玩家目的 → 相关对象/关系 → 权威来源 → 行动结果 → 下一步` 的阅读语义。坐标、派生位置、world units、DTO、selection、renderer 与验证接口由专业 authority 定义。

## 8. 里程碑

1. 产品 PRD、design 与迁移 project 形成稳定专题。
2. 历史 Bevy/EGUI 可读性语义归位并删除重复源文件。
3. 当前 Viewer/视觉 authority 持续提供 surface 与验证证据。

## 9. 风险

- 把视觉强调、派生位置或呈现尺寸误写为权威世界事实。
- 把旧专用控件或完成记录误写成当前 Web 能力。
- 诊断信息或环境装饰重新压过玩家目的、因果和下一步。

## 10. 非目标与专业边界

- 不定义 `commercial_surface`、`fragment_terrain`、`position_source` 或其他 DTO 字段。
- 不冻结派生位置算法、hash/clamp 规则、screen-space LOD、renderer 层级、DOM/WASM/WebGPU 合同、颜色、资产或性能指标。
- 不承诺生产线、吞吐、队列、瓶颈、action preview、设施流、里程碑动画或直接地图操作已经存在。
- 不产生新的玩法控制权、世界规则、资源事实、发布等级或可玩性 claim。
- 2026-05-28 的 player-leverage / production-readability brainstorm 仅是未来输入，不是本分册的当前产品权威。
