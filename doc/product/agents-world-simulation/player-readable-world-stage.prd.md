# 玩家可读的世界舞台

## 文档身份

- 所属产品模块：智能体、世界模拟与交互
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品设计：[玩家可读的世界舞台产品设计](player-readable-world-stage.design.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：2026-10-17
- 专业域权威：[语义定位](../../world-simulator/viewer/viewer-pixel-world-semantic-positioning.prd.md)、[Fragment LOD](../../world-simulator/viewer/viewer-pixel-world-fragment-lod.prd.md)、[玩家可读渲染](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md)、[大世界可视化系统设计](../../world-simulator/viewer/external-agent-world-view.design.md)。

本文定义玩家观察世界时应读懂什么、怎样保持上下文与因果，以及哪些能力必须分别验收。具体DTO、空间算法、渲染管线、平台依赖、容量数值及测试工具属于专业文档；本文不是当前发布报告。

## 设计适用性与生命周期闭合

- 设计判定：`paired-design`。
- 配对设计承接玩家旅程、信息层级、交互状态、来源提示、失败恢复与可访问体验，不另建要求真值。
- 设计适用性理由：世界舞台的阅读顺序、主体和空间来源、交互连续性与恢复解释需要跨状态的产品体验设计。
- 当前实现、目标设计、运行验收和公开支持声明分别判断。历史2D overview/zoom已退役，设计方向和旧截图不能证明现行能力。

## 1. 目标

让玩家在世界现场读懂当前目的、相关Agent/地点/路线、关键阻塞、可采取的下一步，以及已接受行动造成的权威世界反馈。空间舞台采用长期的2D top-down / orthographic像素工业指挥棋盘方向，作为默认primary decision surface；目标、command、receipt、selection上下文相邻且可展开，不以永久多栏挤压世界，也不把命令路径藏到不可发现。

没有已接受的玩家行动或可归因世界后果时，必须明确没有相应进展。世界繁忙、Agent自主活动、动画、renderer正常、诊断可见都不是玩家成功。

产品成功分为三个独立结果：玩家能找到并理解相关对象；能通过合法高层指导影响自己有权控制的Agent；能辨认原行动的结果并继续或恢复。观察成功不等于指导成功，指导被接受不等于世界效果已经发生。

## 2. 首读层级与下一步

- 优先呈现目的、相关行动者/路线、关键blocker和下一决策；世界背景支持理解但不压过行动与因果。
- 结果能关联原目标，并有继续、纠正、中断或恢复入口。主要command从舞台或其紧邻上下文一步可发现；不可用时就近解释原因。
- 相关对象、选中/聚焦、父级或关系上下文可辨。不存在相关对象时显示为空/不可用，不以默认选择制造目标或控制资格。
- 降低密度先隐藏次级标签、细节及装饰，不无差别缩小关键对象或文本；视觉强调不产生新的交互资格。
- 布局、控件、文案、渲染层次与可访问性实现由Viewer/视觉authority决定；Shell的稳定方向不因本文补充而重排。

### 2.1 P0 冻结与 P1-A Agent Context Lite 边界

#### P0 Shell Lite：冻结产品方向

保持世界优先、单一Next Move主动作、从属Objective/Player Leverage、单一因果回执及按需展开的上下文。冻结不是实现完成或公开发行结论。允许来源时效、权限、因果唯一性、窄屏和恢复的focused修正，不以常驻三栏、底部hotbar或大范围HUD重排作为前置。

#### P1 有序 rollout 与阶段依赖

1. **P1-A Agent Context Lite**：一个当前选中Agent的身份、状态、来源/时效、适用目标、下一步、blocker、Activity、Intent、Action Receipt与权限。
2. **P1-B World semantic presentation**：在P1-A语义成立后扩展有权威身份、关系种类/状态、路线或位置来源的世界表达；邻近、同屏、文本不能补齐关系，不扩展Agent控制权。
3. **P1-C Major World Event**：事件具有身份、种类/严重度、来源、因果锚点、时效、生命周期和重放/重组语义后，再设计注意力。事件不是Action Receipt，不建立第二条玩家因果链。

当前专业实现收据记载P1-B snapshot派生stage cues和P1-C crisis-only additive projection/ambient feed；crisis没有空间anchor，因而没有stage marker/highlight。生产Major Event只消费显式runtime/operator audience policy，默认Unknown fail closed，明确Public/Restricted才披露。登录、选择、控制或Director权限都不产生该policy。局部实现收据不证明完整P1或公开发行。

#### P1-A：Agent-only Context Lite

选中只建立解释上下文，不等于绑定、认领或可控制；授权沿用[Agent对话与Prompt控制](agent-conversation-and-prompt-control.prd.md)和[Agent自治、委托与责任连续性](agent-authority-ownership-and-accountability.prd.md)。

只组合已经发布且明确适用于该Agent的可读身份、公开且有来源的所在/父级上下文与状态。Objective、Next Move、blocker、Player Leverage、执行状态只有明确绑定该Agent才可作为其上下文；global summary/primary intent仍属于玩家，不能重命名为Agent的私有目标。

Activity、权威Intent、玩家local_pending草稿、Action Receipt与通信反馈独立呈现。Intent需要Agent/目标匹配及明确接受/阻断语义；草稿不是Agent当前Intent。回执只引用现有Player/Cinematic单一因果锚点；没有绑定回执时保持无回执，不从Feed、距离、同屏、文本、客户端时间或raw object推导成功、关系、ownership、计划、资源或能力。

Capability需要适用主体、明确来源、权限、时效和恢复路径。缺失时不可用/待同步或省略，不用全局推荐或其他对象的能力填充。Facility、Territory、Organization、Depot、Module、Location不继承Agent的语义，等待各自权威合同。

#### P1-A 验收映射

| ID | 产品承诺 | 组合验收 |
| --- | --- | --- |
| P1A-CTX-01 | 从世界/目标选中Agent后，0–1次情境转换内辨认身份、适用状态、目的和下一决策；缺字段明确为空。 | RW-1、RW-6 |
| P1A-CTX-02 | 当前确认、最近已知、冲突、未知或失效不被抹平；影响行动时有刷新、等待、改道或停止路径。 | RW-8、RW-9 |
| P1A-CTX-03 | Activity、Intent、草稿、单一因果回执与非因果反馈独立；选择切换不改写原行动归属。 | RW-3、RW-4 |
| P1A-CTX-04 | capability只来自明确适用的权威投影与权限；global summary、选中/可见/邻近不创造控制权。 | RW-2、RW-6 |
| P1A-CTX-05 | 世界优先、诊断次级；目的、阻塞、回执和恢复在窄屏、CJK、辅助技术下仍可理解。 | RW-1、RW-4、RW-5 |

#### P1-A 明确非目标

不重做P0 Shell；不建立全实体通用Inspector；不一次迁移Market、War、Governance、Depot或完整Programs/Protocols；不增加global inventory、capability hotbar、直接移动/攻击/采集；不改变runtime/protocol、经济、治理、ownership或数值规则；不把plan/rationale/ETA/memory/provider/prompt/auth诊断变成默认玩家详情。

<a id="stage-subject-identity"></a>
### 2.2 观察焦点、行动对象与回执归属

本节细化REQ-AGENT-STAGE-001/005，不新增控制方式。

| 语义 | 合法来源与行为 | 禁止的替代 |
| --- | --- | --- |
| 观察焦点 | 玩家选择、授权搜索定位；初始构图可采用明确为展示用途的默认视点。 | 把列表首项、默认视点或可见对象称为玩家已控制对象。 |
| 行动对象 | 当前权威动作/指导上下文，以及经过适用授权校验的明确对象选择。提交后属于原操作。 | 凭默认焦点、推荐变化或列表顺序补齐、替换提交目标。 |
| 回执归属 | 原操作、Agent/任务/目标及权威结果的明确关联；无主体的玩家级结果保留玩家级。 | 借当前selection、首个Agent、最新推荐或文本猜测主体。 |

玩家提交A的指导后查看B，A的延迟结果仍属于A；选择B不能把该结果改成B的成功，也不能自动重发动作。B的上下文没有绑定结果时明确无回执，玩家级原回执仍可追溯。多条历史记录可以查询，但当前因果解释仍引用同一权威操作，不复制成多次效果。

未取得明确行动对象时，可保留只读浏览或未提交草稿，并提供选定合法对象/重新同步入口；不能为让按钮可点击而借用fallback对象。观察权撤销时删除不再获准披露的细节，不以最近已知为由保留。

## 3. 空间关系与来源

不承诺所有对象都有精确坐标。派生/抽象位置必须与权威世界位置可区分，不能宣称真实距离、可执行路线、资源地点或控制资格。marker/halo/标签可为可读性放大，但表现尺寸不是物理尺寸；获准权威文本/数值优先。

关系线、箭头、热点只解释已支持关系或效果，不从邻近、同屏或装饰产生关系、因果、所有权或动作。terrain/fragment blocks只提供空间/材料语境，细节增加不产生hover/selection、edit/harvest/build能力。多尺度保持目标、行动者、关键路线、blocker、下一步与返回路径，不是退役2D/zoom功能已交付声明。

### 3.0 产业关系与空间层级的持续要求

权威数据支持时，应识别工厂/配方/产品/物流节点，区分物料/电力/数据流向，以及有来源的吞吐、损耗、瓶颈、拥塞与告警；根因线索能定位location/Agent/module。分层档案支持时，符号区分R1~R5和bootstrap/scale/governance，状态独立且不只靠颜色。

World/Region/Node分别突出热区/主干流、区域关系、配方/库存细节。缺数据显示未知/不可用，推断根因与确认事实分开。当前routes、hotspots、HUD不证明完整产业图谱、吞吐流、分层符号或三档semantic zoom已交付。

### 3.1 观察范围、时效与未知边界

动态对象、位置、路线、容量、部署和状态区分当前授权范围内确认、仍可披露但可能变化的最近已知，以及未取得/失效/冲突。公共已结算结果是历史事实，不自动授予实时经营情报、精确位置或控制权；动态观察也不能改写已结算历史。

范围外、未侦察、加载失败、缓存过期、组织归属、默认选择或界面可达不意味着全知。几何来源与信息时效独立：曾经权威的坐标可以成为最近已知，派生位置不能因刚刷新变成精确坐标。

影响当前行动的不确定信息提供刷新、等待、改道、停止或安全返回。专业域决定可见性、TTL、授权、DTO；玩家与Agent不要求共享相同原始observation。

#### 3.1.1 来源冲突、刷新竞态与行动降级

- 冲突先保留和解释，不以客户端时间、地图排序或最后到达的响应选真值；矛盾的当前观察显示待重新确认。
- 刷新中旧信息最多是仍被授权披露的最近已知；旧请求晚到不覆盖新的已确认状态。重复刷新/重连/跨入口不产生新观察资格或第二世界效果。
- 只读浏览保留来源说明；无副作用预览标注不确定性。影响资源、权限、路线承诺、设施状态或他人权利的生效操作，由对应权威在实际接受/生效边界重新确认，或明确拒绝/重新预览。
- 未提交草稿不等于激活目标；预览有效不保证提交有效，上下文改变后不自动批准变化方案。
- Agent可提出待核验建议，不把缓存/推断/冲突一方表述为确认事实并自动提交高后果行动。
- 已授权stop、撤销、原结果回查遵守各自控制合同，不因renderer失败或观察陈旧一律禁用，也不能越过服务器授权。
- 反馈说明原因、仍可信内容、受阻操作与恢复。结果不确定时先回查原操作，不以新键重做；专业trace可对账显示分类、重新校验与最终结果。

### 3.2 可探索的大世界与网页/桌面客户端

<a id="world-exploration-surfaces"></a>
长期旅程是世界概览→区域/Fragment→授权搜索/定位→对象检查→权威结果→合法指导→返回总览/当前目标，而非只看Agent面板或日志。

- 平移/缩放或等价尺度切换、明确返回总览/目标，不靠无边界拖动寻找对象。
- 搜索、选择、检查和合法跟随只覆盖获准且有数据的Agent/地点/设施/路线/事件；缺事件anchor时在Feed检查，不造marker。
- 高密度、聚合、过滤、部分加载、视点裁剪与权威删除不同；不在画面内不等于世界中不存在。
- 重叠对象可经可访问候选列表可靠选择；跟随可退出，手动浏览不被抢回视点。来源/权限失效停止追踪，不默认切换其他Agent。
- 浏览、选择、跟随、过滤、图层和下钻只是观察；自己的合法Agent可进入已有目标/Prompt/委托；地图点击不直接采集、建造、移动。
- 正式Web和受支持原生客户端分别在自身界面交互；桌面可内嵌同一Web舞台，系统浏览器跳转、静态图或Launcher按钮不算客户端内能力。
- 断连、局部加载失败、renderer启动/运行中失效时，保留仍获准的历史结果、目标、可恢复选择与文本；明确陈旧画面及刷新/返回，不把空画布解释为空世界。
- 移动/触控、键盘、CJK和低高度保持同一概念路径。平台和质量由专业权威逐组合验证，Web成功不推导其他端成功。

## 4. 玩家因果与诊断边界

只有已接受意图及可归因世界后果表达玩家影响。accepted/submitted/queued不是完成；完成但无进展与真实进展不同；阻断、拒绝与结果待确认各有恢复。重放不表达新收益，重组后的结果按权威状态解释，旧动画不能保留成功。

目的、blocker、回执和恢复有文本或等价语义。renderer、source、原始DTO、性能和测试控制次级且按需；文本降级不另建世界渲染/真值。mock、DOM、raster、GPU截图和真实结果分别证明各自范围。

## 5. 叶级产品要求与验收

五项要求沿用原Trace ID，细化不重复发号。设计状态仅指目标合同覆盖；实现/独立评审/运行结果分别记录，未核实不能解释为已实现或必然未实现。

<a id="req-agent-stage-001"></a>
### REQ-AGENT-STAGE-001：世界舞台必须先呈现目的与因果

Trace ID: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616

- 功能点：目标、相关对象、blocker、下一步及原行动结果优先；观察焦点不补行动/回执归属。
- 系统设计状态：已覆盖
- 系统设计依据：[共享舞台与主体边界](../../world-simulator/viewer/external-agent-world-view.design.md#view-scene)承接目标架构、原操作和显示分层，专业渲染及质量条款共同细化；不代表实现通过。
- 实现状态：已实现待验证
- 实现与验证依据：本轮主体绑定隔离已实现，待完整测试；焦点、推荐对象、原回执及显式空主体分离。实现与局部测试见 [PR #4825](https://github.com/nil-studio/oasis7/pull/4825) 和[本轮摘要](#stage-implementation-status)。
- 核对日期与基线：2026-10-17；实现基线 `bb27f5c033674419d24575872bd64e8a0c44a4d0`；局部测试不代签完整验收。
- 剩余范围：同候选端到端因果/理解、双主体延迟结果及正式独立试玩。
- 验收：[AC-AGENT-STAGE-001](#ac-agent-stage-001)。

<a id="ac-agent-stage-001"></a>
### AC-AGENT-STAGE-001：玩家能从舞台读到下一决策

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616

- 覆盖要求：[REQ-AGENT-STAGE-001](#req-agent-stage-001)。
- 场景与结果：目标、阻塞、下一步及原结果在密度变化后可读，继续/纠正/中断/恢复可达。A在途切B、过滤或列表重排，结果仍归A原操作；无行动/回执、玩家级无Agent结果和仅ambient不补造主体。
- 证据边界：按[首读测试](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-readability)和原操作对账，不以截图存在判产品通过。

<a id="req-agent-stage-002"></a>
### REQ-AGENT-STAGE-002：动态观察必须保留来源、时效与未知

Trace ID: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3

- 功能点：当前/最近已知/未知/冲突、来源范围、提交时重新确认与恢复。
- 系统设计状态：已覆盖
- 系统设计依据：[观察到生效的权威重验](../../world-simulator/viewer/external-agent-world-view.design.md#view-admission)承接只读/预览/高后果生效和安全控制，查询/适配保留来源与竞态语义。
- 实现状态：部分实现
- 实现与验证依据：本轮适用 Web 子集已实现，待完整测试；范围、实现 PR、局部测试及未实现边界见[本轮摘要](#stage-implementation-status)。
- 核对日期与基线：2026-10-17；实现基线 `bb27f5c033674419d24575872bd64e8a0c44a4d0`；局部测试不代签完整验收。
- 剩余范围：真实Context/fence/控制版本接线、原结果恢复、权限/提交竞态与客户端验收。
- 验收：[AC-AGENT-STAGE-002](#ac-agent-stage-002)。

<a id="ac-agent-stage-002"></a>
### AC-AGENT-STAGE-002：陈旧或冲突观察不会代签真值

Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3

- 覆盖要求：[REQ-AGENT-STAGE-002](#req-agent-stage-002)。
- 场景与结果：旧响应、冲突、刷新撤权、提交前过期/转让/目标变化不覆盖新事实，不先绘制后鉴权。低后果预览标不确定，高后果由权威重验/阻断；stop/原结果查询按自己的授权可达，未知结果不盲重做。
- 证据边界：显示cut、分类、原操作与权威拒绝/结果能对账，不由Viewer自报current代签。

<a id="req-agent-stage-003"></a>
### REQ-AGENT-STAGE-003：大世界必须可多尺度探索

Trace ID: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf

- 功能点：授权总览/区域/Fragment/对象浏览、平移/缩放、搜索、选择/跟随、返回目标和高密度可用性。
- 系统设计状态：已覆盖
- 系统设计依据：[空间查询与探索](../../world-simulator/viewer/external-agent-world-view.design.md#view-query)组合适配/partial/容量合同；Fragment LOD仅承接背景细节，不替代完整探索。
- 实现状态：部分实现
- 实现与验证依据：本轮适用 Web 子集已实现，待完整测试；范围、实现 PR、局部测试及未实现边界见[本轮摘要](#stage-implementation-status)。
- 核对日期与基线：2026-10-17；实现基线 `bb27f5c033674419d24575872bd64e8a0c44a4d0`；局部测试不代签完整验收。
- 剩余范围：授权空间投影、分页/聚合、可靠交互、密集质量与真实浏览器证据。
- 验收：[AC-AGENT-STAGE-003](#ac-agent-stage-003)。

<a id="ac-agent-stage-003"></a>
### AC-AGENT-STAGE-003：世界概览至对象再回到当前任务

Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf

- 覆盖要求：[REQ-AGENT-STAGE-003](#req-agent-stage-003)。
- 给定：多区域、部分加载、无精确坐标、未知和重叠对象。
- 当：浏览、定位、选择、跟随/退出及返回目标。
- 则：身份、层级、来源、blocker和目标连续；分页不当全量删除，聚合/过滤不制造不存在；稀疏/密集和输入方式分别验收。

<a id="req-agent-stage-004"></a>
### REQ-AGENT-STAGE-004：网页和原生桌面客户端都要能实际探索世界

Trace ID: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a

- 功能点：声明支持的Web/原生端各自在界面内浏览、定位、检查、重连与图形恢复。
- 系统设计状态：已覆盖
- 系统设计依据：[原生容器与支持边界](../../world-simulator/viewer/external-agent-world-view.design.md#view-native)组合共享Viewer和恢复合同；目标平台表不是当前支持声明。
- 实现状态：部分实现
- 实现与验证依据：本轮适用 Web 子集已实现，待完整测试；范围、实现 PR、局部测试及未实现边界见[本轮摘要](#stage-implementation-status)。
- 核对日期与基线：2026-10-17；实现基线 `bb27f5c033674419d24575872bd64e8a0c44a4d0`；局部测试不代签完整验收。
- 剩余范围：实际WebView依赖/最低版本锁定、OS/架构发行资产、安全/恢复与质量验证。
- 验收：[AC-AGENT-STAGE-004](#ac-agent-stage-004)。

<a id="ac-agent-stage-004"></a>
### AC-AGENT-STAGE-004：两个 surface 分别有可交互世界证据

Trace refs: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a

- 覆盖要求：[REQ-AGENT-STAGE-004](#req-agent-stage-004)。
- 给定：同一真实世界、Agent成果和固定Web/native候选。
- 当：各端缩放、定位、检查、断连回访，并经历启动失败和ready后图形失效。
- 则：本界面交互及结果同义，权限不扩大；系统浏览器、静态图或另一平台通过不代签。

<a id="req-agent-stage-005"></a>
### REQ-AGENT-STAGE-005：地图只能进入合法间接指导

Trace ID: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096

- 功能点：有权Agent的目标/Prompt/委托与原回执衔接；观察无世界副作用。
- 系统设计状态：已覆盖
- 系统设计依据：[合法高层指导](../../world-simulator/viewer/external-agent-world-view.design.md#view-guidance)组合主体固定、服务器重验和原操作恢复，不建立地图直接行动。
- 实现状态：部分实现
- 实现与验证依据：本轮适用 Web 子集已实现，待完整测试；范围、实现 PR、局部测试及未实现边界见[本轮摘要](#stage-implementation-status)。
- 核对日期与基线：2026-10-17；实现基线 `bb27f5c033674419d24575872bd64e8a0c44a4d0`；局部测试不代签完整验收。
- 剩余范围：服务器接线、原操作幂等恢复、无权/转让/主体切换负例与真实世界结果。
- 验收：[AC-AGENT-STAGE-005](#ac-agent-stage-005)。

<a id="ac-agent-stage-005"></a>
### AC-AGENT-STAGE-005：选中对象不授予世界控制

Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096

- 覆盖要求：[REQ-AGENT-STAGE-005](#req-agent-stage-005)。
- 给定：自己的Agent、可见但无权Agent、地形和pending/committed操作。
- 当：浏览/过滤/选择，再对合法Agent提交高层指导，并注入转让/撤权/旧草稿/断连。
- 则：观察不提交世界action；保存、接受、生效和后续结果分开；主体不变且无第二次效果。

<a id="stage-implementation-status"></a>
### 本轮实现状态：已实现，待完整测试

本轮主体绑定、显示恢复与 Agent/Location 探索切片的实现状态为 **已实现待验证**（已实现，待完整测试）。实现与逐项验证见 [PR #4825](https://github.com/nil-studio/oasis7/pull/4825)，实现基线 `bb27f5c033674419d24575872bd64e8a0c44a4d0`。本摘要不将下列完整要求或尚未实现的范围改判为已完成。

| 本轮范围 | 已实现内容 | 待完整测试与剩余边界 |
| --- | --- | --- |
| 主体绑定（001/005） | 观察焦点、推荐行动对象和原回执主体独立；玩家级空主体保留；选择变化不补写原回执。 | 双主体真实指导、延迟结果、权限变化及世界结果组合验收。 |
| 来源与恢复（002/004 Web 子集） | 已实现，待完整测试。远端获准投影校验来源/切点与实际 Runtime 一致性；授权投影保留来源/时效；冻结续页整体降为最近已知；作用域切换关联快照恢复，旧异步会话隔离；图形失败保留获准文本并显式重试。 | 私有账号切换、真实图形上下文失效恢复、独立正式游戏 Skill 试玩。 |
| 空间探索（003 Web 子集） | 已实现，待完整测试。同 Viewer 连接有界分页、搜索、Agent/Location/设施只读检查、显式路线投影、Rust 视口查询与空间索引、定位/跟随/停止及返回总览/目标。 | 真实远端、复杂产业与事件组合、高密度与长期容量证据；原生容器仍未实现。 |

远端来源绑定、设施/路线投影与视口查询/空间索引已在 [PR #4834](https://github.com/nil-studio/oasis7/pull/4834) 合入。本轮增量更新与明确删除通知、工业多尺度聚合、显式事件锚点的实现状态为 **已实现，待完整测试**：只在同范围、同授权来源基线上应用原子更新；区分离开投影与已证实回收；设施群在授权筛选后计数并可下钻，pin 保留独立身份；事件使用原始显式坐标并按原身份回链 Feed。仅消费现有获准投影，来源不足或未知位置继续保留文本。事件锚点覆盖最近有界 typed 候选，不等于完整事件历史；无锚点的危机等事件留在 Feed。真实远端、正式试玩和容量结果仍须独立测试，不能从索引或聚合存在推定达标。原生容器仍不属于已实现范围。

局部测试、WebGL/WASM 构建和桌面/窄屏本地无 LLM 浏览器检查结果见对应 PR；这些结果不代签完整产品验收。缺私有测试世界和 GameClient 设备配置，正式独立试玩保持 blocked/not run；用户已明确要求保留缺口并推进合入。完整 REQ 的状态仍按各卡实际范围判断，不将未实现范围写成仅待测试。

## 5.1 叶级 owner、authority、evidence 与 test tier 追踪

| REQ / AC关系 | 专业owner与边界 | 专业权威 | 验证证据与 test tier |
| --- | --- | --- | --- |
| [REQ-AGENT-STAGE-001](#req-agent-stage-001) / [AC-AGENT-STAGE-001](#ac-agent-stage-001) | product/visual首读；Viewer投影；Runtime/gameplay因果；QA理解。 | [玩家可读渲染](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md#41-commercial-surface-dto) | PW-IDENTITY/READABILITY；确定性test_tier_required，真实因果/理解test_tier_full。 |
| [REQ-AGENT-STAGE-002](#req-agent-stage-002) / [AC-AGENT-STAGE-002](#ac-agent-stage-002) | Runtime/Agent校验，Viewer表达，QA竞态。 | [观察到提交](../../world-simulator/viewer/external-agent-world-view.design.md#view-admission) | PW-FRESHNESS；确定性及test_tier_full实际组合分别留证。 |
| [REQ-AGENT-STAGE-003](#req-agent-stage-003) / [AC-AGENT-STAGE-003](#ac-agent-stage-003) | Viewer/visual空间交互；Runtime授权数据；QA容量。 | [空间适配](../../world-simulator/viewer/external-agent-world-view.design.md#view-adapter) | PW-EXPLORATION/PERFORMANCE；完整范围test_tier_full。 |
| [REQ-AGENT-STAGE-004](#req-agent-stage-004) / [AC-AGENT-STAGE-004](#ac-agent-stage-004) | Viewer/Launcher容器发行；QA逐平台。 | [Native](../../world-simulator/viewer/external-agent-world-view.design.md#view-native) | PW-RECOVERY/PLATFORM；test_tier_full，不相互代签。 |
| [REQ-AGENT-STAGE-005](#req-agent-stage-005) / [AC-AGENT-STAGE-005](#ac-agent-stage-005) | Agent/Runtime指导权限结果；Viewer不另写世界。 | [指导](../../world-simulator/viewer/external-agent-world-view.design.md#view-guidance) | PW-IDENTITY/FRESHNESS；权限 test_tier_required，真实结果 test_tier_full。 |

## 5.2 组合验收

- RW-1：不读诊断即可识别世界、目标、相关对象/路线、blocker、下一步。
- RW-2：空间关系可读，派生/抽象位置不冒充精度、资源事实或能力。
- RW-3：区分原玩家行动可归因结果与环境/自治活动，并可继续/纠正/中断/恢复。
- RW-4：目的/行动/反馈优先，诊断次级且可达。
- RW-5：证据关联当前gameplay/runtime/Agent/Viewer/QA候选，单图/mock/fallback不单独成立产品结论。
- RW-6：舞台primary read与相邻上下文连续，command可发现，不以详情取代世界。
- RW-7：密度变化先退次级标签，terrain非edit/harvest/build affordance。
- RW-8：公共结果、当前确认、最近已知、未知/失效分开，非当前信息有恢复路径。
- RW-9：冲突/旧响应/提交前失效不代签真值，预览与高后果重验分开。
- RW-10：有权总览至区域/对象/事件再返回目标，聚合/未加载不伪装全知或空世界。
- RW-11：Web和受支持原生各自交互，不由浏览器跳转或截图代签。
- RW-12：观察衔接合法间接指导和原回执，不直接操控世界。

### 5.2.1 验收权威与证据边界

场景、指标、视口、密度、输入和平台组合见[专项验证手册](../../testing/manual/pixel-world-visualization-validation.manual.md)。普通CI先覆盖确定性合同，不强制每个文档PR跑付费模型/GPU/真人。真实玩法、S6、L5、发行和公开声明各按原authority；缺证据标未验证，不降低标准制造通过。

## 6. 范围

覆盖首读、对象/关系、比例/来源、因果、观察式探索、两端同义、文本与故障恢复。renderer、资产、世界规则和写接口不在本产品分册定义。

## 7. 接口 / 数据

阅读语义是玩家目的→相关对象/关系→来源与适用性→原行动结果→下一步。产品不冻结DTO/坐标/TTL/相机/缓存，专业适配须保留上述语义而非只保留绘图位置。

## 8. 里程碑

保持P0与P1-A/B/C依赖。先统一主体/来源，再接有界读模型/探索，最后扩大原生与容量声明；各项以实现和验证为准，不按文档合入宣称完成。

## 9. 风险

默认焦点污染行动/回执；partial误作全量；权限缩小时泄漏旧详情；性能降级丢关键对象；目标架构/旧截图冒充当前交付。专业合同须有对应负例。

## 10. 非目标与专业边界

不新增直接世界动作、控制权、资源事实或发布等级；不恢复第二Viewer、旧3D/2D工具链；不冻结commercial_surface、fragment_terrain、position_source、shader、LOD阈值、颜色、资产和性能数值。产业图谱、吞吐/队列、设施流、动画、native/semantic zoom目标不等于已存在。2026-05-28 brainstorm只是候选，不是当前authority。

## 11. 未决问题与假设

运行实现、平台依赖锁定、真实可读性和容量结果仍待交付，设计覆盖不消除实施/独立评审缺口。缺当前授权、位置、关系、因果或时效时保持未知/待确认并提供合法恢复，不用视觉邻近、首项或缓存补事实。
