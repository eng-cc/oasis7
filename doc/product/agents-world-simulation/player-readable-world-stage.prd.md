# 玩家可读的世界舞台

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/player-readable-world-stage.design.md`](player-readable-world-stage.design.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`语义定位`](../../world-simulator/viewer/viewer-pixel-world-semantic-positioning.prd.md)、[`Fragment LOD`](../../world-simulator/viewer/viewer-pixel-world-fragment-lod.prd.md)、[`玩家因果优先的渲染闭环`](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md)

本文是长期产品分册，定义玩家观察 Agent 与世界模拟时的首读层级、空间关系、可归因因果与诊断边界。它不指定 Viewer 组件、DTO、派生算法、LOD 阈值、渲染管线、视觉资产或当前发布结论。

## 1. 目标

玩家进入正式世界表面后，应先读懂当前目标、相关 Agent 与地点或路线、关键阻塞、可采取的下一步，以及已接受行动造成的可理解世界反馈。环境、地形与世界活动提供必要背景，但不能压过玩家行动和因果。

长期方向是把空间化的活世界舞台作为默认 primary decision surface：它采用 2D top-down / orthographic 的像素工业指挥棋盘语法，首先帮助玩家理解世界、目标、相关行动者/路线、阻塞与下一步。目标、command、receipt 与 selection 的情境管理保持相邻且可展开的次级层，不能与舞台争夺首读，也不能把命令路径藏到不可发现。

该方向不是当前已交付的 2D、top-down、orthographic 或 semantic zoom claim。历史 2D overview/zoom 已退役，当前 Viewer surface 和验证证据仍由专业 authority 单独裁定；本分册不以历史实现、设计方向或截图替代当前 readiness。

没有已接受的玩家行动或可归因的权威世界变化时，产品必须明确这一事实；世界仍在活动、画面信息丰富、renderer 正常或调试数据可见，都不能被表达成玩家已经取得进展。

## 2. 首读层级与下一步

- 首读层优先呈现当前目的、相关行动者或路线、关键 blocker 和下一决策，而不是默认展示技术状态、原始数据或环境计数。
- 玩家能够把结果与当前目标联系起来，并找到继续、纠正、中断或恢复的下一步。
- 世界背景需要支持理解场景和关系，但不得以信息密度或视觉权重掩盖 Agent、目标、阻塞与玩家可施加的影响。
- 舞台是默认 primary decision surface；目标、command、receipt 与 selection 的情境管理可在相邻区域或按需展开，但保持次级，不得在首读层取代世界关系。玩家始终可以从舞台或其紧邻上下文发现受支持 command 的进入路径。
- 产品层只约束玩家能够读懂什么；具体布局、文案结构、交互控件、渲染层次与可访问性实现继续由 Viewer 与视觉交互专业权威拥有。
- 玩家能够辨认当前相关对象及其选中或聚焦状态；没有相关 Agent、地点或路线时，surface 必须诚实表达为空或不可用，不能通过默认选中制造虚假目标。
- 世界背景可以为降低噪声而简化，但相关对象、其关系或父级上下文、选中或聚焦状态与可用性仍须可辨认；视觉强调不会产生新的交互资格。

## 3. 空间关系与事实来源

- 玩家需要读懂当前相关 Agent、地点、路线、目标或 blocker 之间的关系，但产品不承诺所有对象都有精确坐标。
- 专业表面可以为稳定表达关系使用派生或抽象位置，但必须区分权威世界位置与仅用于呈现的关系位置；后者不得被包装成精确坐标、权威资源地点或可执行世界事实。
- 派生呈现不会产生新的控制权、目标权或世界规则。玩家动作仍通过受控入口提交，并由同一权威运行时接受、拒绝或产生替代结果。
- 地形、材料或环境结构可以作为世界上下文，但其详细度不得自动赋予 hover、selection、采集、建造或其他交互能力。
- 长期 2D top-down / orthographic 像素工业指挥棋盘的空间语法用于解释关系、工业活动和指挥上下文；terrain 或 fragment blocks 只提供语境，绝不暗示玩家可直接 edit、harvest、build 或绕过受控 command 路径。
- 地图、关系线、方向标记、halo 或其他视觉辅助只用于解释已受支持的关系与效果，不会生成控制权、所有权、因果事实、精确位置或可执行能力。
- 长期语义缩放在不同密度/尺度下优先保留当前目标、相关行动者或路线、关键 blocker 与下一步；它应先折叠次级 labels 和非关键细节，而非无差别缩小整个决策面到不可读。该方向不承诺当前存在 2D 地图、自动缩放、专用 overview 控件或已交付 semantic zoom。

### 3.1 观察范围、时效与未知边界

- 正式玩家表面若以观察、侦察或缓存情报说明动态世界对象、位置、路线、容量、部署或近期状态，必须区分：当前在适用观察范围内确认、仍有效但可能已变化的最近已知，以及尚未取得或已失效。范围外或未知对象不得因地图、默认选择、历史缓存、组织归属或界面可达而被表现成当前可见的精确世界事实。
- 已成立的公共规则、已结算结果和依法可公开的安全事实可以作为公共事实呈现；它们不因此授予实时经营情报、精确位置、控制权或额外行动资格。公共事实与观察所得的动态情报必须保留不同的来源语义。
- 会实质影响当前行动的最近已知或不确定信息，必须保留适用范围和时效边界，并让玩家可理解地选择刷新、等待、改道或停止；过期观察不能静默驱动长期行动，也不能因自动化而绕过权威校验。
- 本节约束产品事实边界，不要求玩家与 Agent 获得相同原始 observation 或暴露实现字段；专业域仍决定可见性规则、缓存时长、授权、DTO、文案和交互表达。

#### 3.1.1 来源冲突、刷新竞态与行动降级

动态情报可能同时来自已结算公共事实、当前授权观察、最近已知缓存或 Agent 的推断。来源之间发生冲突时，产品语义必须先保留冲突，再决定是否允许行动，不能由客户端时间戳、地图排序、默认选中或“最新收到的响应”静默选出一个真值。

- 已结算公共事实继续作为历史结果的依据；它不能单独覆盖更晚的动态观察，也不能被动态观察改写。对同一动态属性的多个当前观察若互相矛盾，surface 应标记为“信息冲突/待重新确认”，并同时保留各自的适用范围和时效语义。
- 冲突、范围不明、刷新响应落后于当前已显示状态，或观察在提交前已超过适用时效时，低后果动作最多只能作为明确标注不确定性的预览；任何会改变资源、权限、路线承诺、设施状态或他人权利的动作，都必须先重新取得当前权威确认，或提供等待、改道、停止和回到安全入口中的适用选择。
- 刷新期间，旧观察可以继续作为“最近已知”上下文，但不能被表示为当前确认；较晚开始而较早返回的刷新结果不得覆盖较新的已确认结果。重复刷新、重连或跨入口查看只产生一次可追溯的当前判断，不产生第二份观察资格、优先权或世界效果。
- Agent 可以基于不确定性提出待核验建议，但不能把推断、冲突中的某一来源或最近已知缓存表述为已确认事实，也不能据此自动提交高后果行动。玩家必须能看到系统是在使用当前确认、最近已知、冲突待核验还是未知，并知道下一步会刷新、等待、改道、停止或重新确认。
- 该降级不是隐藏失败：反馈至少说明冲突/过期/范围不足的原因、当前仍可依赖的事实、被阻断的行动类别和恢复入口。专业 authority 应能用同一条观察/行动 trace 证明显示的来源分类、提交时重新校验结果及最终接受或拒绝原因；产品层不冻结 trace 字段、排序算法或 TTL。

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
- RW-6：长期空间舞台样例保持世界、目标、相关行动者/路线、blocker 与下一步的 primary read；相邻 target/command/receipt/selection 管理可展开但不隐藏受支持的 command path，也不替代舞台成为默认决策面。
- RW-7：长期密度变化样例证明 semantic zoom 先退次级 labels/细节，持续保留目标、相关行动者或路线、blocker 和下一步；terrain/blocks 只解释语境，不表达直接 edit/harvest/build affordance。
- RW-8：代表性动态信息场景证明正式玩家表面能区分公共已结算事实、当前确认、最近已知和未知/失效观察；范围外或缓存对象不会被表现为当前精确事实，且影响行动的非当前信息提供刷新、等待、改道或停止路径。
- RW-9：同一动态对象出现互相矛盾的观察、刷新竞态（旧响应晚到）或提交前时效失效时，surface 保留冲突/来源/时效语义，不以客户端“最新响应”代签真值；低后果动作至多提供带不确定性标注的预览，高后果动作必须重新取得当前权威确认或明确阻断，并能读到冲突原因与下一步。

### 5.1 验收权威与证据边界

| 产品承诺 | 专业 owner | 专业域权威 | 证据边界 | 测试层级 |
| --- | --- | --- | --- | --- |
| 首读层级与空间关系来源诚实 | viewer_engineer / game_visual_interaction_designer / qa_engineer | `doc/world-simulator/viewer/viewer-pixel-world-semantic-positioning.prd.md`; `doc/world-simulator/viewer/viewer-pixel-world-fragment-lod.prd.md`; `doc/world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md` | 正式玩家表面中的目标、关系、阻塞与下一步可读性，以及权威位置和派生呈现的区分；不复制 DTO、阈值或 renderer 合同 | test_tier_required |
| 可归因玩家因果与恢复路径 | gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 以已接受意图、权威世界结果、主要因果和下一决策或恢复路径组成端到端证据；环境活动不能代签玩家影响 | test_tier_full |
| 长期舞台优先、语义缩放与非直接编辑边界 | game_visual_interaction_designer / viewer_engineer / gameplay_designer / qa_engineer | `doc/world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md`; `doc/world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md`; `doc/testing/prd.md` | 目标/行动者/路线/blocker/下一步在密度变化下的保留、次级 label 收敛、command path 可发现性与 terrain 非 affordance 的未来验证；不代签当前 2D/zoom readiness | test_tier_required |
| 动态观察的范围、时效与未知边界（RW-8） | producer_system_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 同一动态对象在当前范围、缓存未过期、缓存过期或未取得观察时的来源/状态分层；范围外不泄露为当前事实，影响行动的非当前信息具有刷新、等待、改道或停止路径；不复制可见性算法、TTL、DTO 或 UI 合同 | test_tier_required |
| 来源冲突、刷新竞态与行动降级（RW-9） | producer_system_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 同一对象的冲突观察、旧刷新响应覆盖尝试、提交前过期与重连/重复刷新负例；验证公共事实、当前观察、最近已知、未知/冲突的可读来源语义，高后果动作重新校验或阻断，且 trace 能对账显示分类与最终接受/拒绝原因；不复制字段、排序或 TTL 合同 | test_tier_full |

## 6. 范围

覆盖正式玩家 surface 的首读层级、相关对象与关系辨认、空间与比例诚实、可归因因果、支持文本和诊断降级。不覆盖具体 Viewer 控件、renderer、视觉资产或专业实现。

## 7. 接口 / 数据

产品层只定义 `玩家目的 → 相关对象/关系 → 权威来源 → 行动结果 → 下一步` 的阅读语义。坐标、派生位置、world units、DTO、selection、renderer 与验证接口由专业 authority 定义。

## 8. 里程碑

1. 产品 PRD 与 design 形成可从模块入口进入的稳定专题。
2. 历史 Bevy/EGUI 可读性语义归位并删除重复源文件。
3. 当前 Viewer/视觉 authority 持续提供 surface 与验证证据。

## 9. 风险

- 把视觉强调、派生位置或呈现尺寸误写为权威世界事实。
- 把旧专用控件或完成记录误写成当前 Web 能力。
- 诊断信息或环境装饰重新压过玩家目的、因果和下一步。

## 10. 非目标与专业边界

- 不定义 `commercial_surface`、`fragment_terrain`、`position_source` 或其他 DTO 字段。
- 不冻结派生位置算法、hash/clamp 规则、screen-space LOD、renderer 层级、DOM/WASM/WebGPU 合同、颜色、资产或性能指标。
- 不承诺生产线、吞吐、队列、瓶颈、action preview、设施流、里程碑动画、直接地图操作、2D overview、top-down/orthographic board 或 semantic zoom 已经存在；历史 2D overview/zoom 退役不能被反向引用为当前能力。
- 不产生新的玩法控制权、世界规则、资源事实、发布等级或可玩性 claim。
- 2026-05-28 的 player-leverage / production-readability brainstorm 仅是未来输入，不是本分册的当前产品权威。
