# 玩家可读的世界舞台

## 文档身份

- 所属产品模块：智能体、世界模拟与交互。
- 上位产品 PRD：[`prd.md`](prd.md)。
- 配对产品设计：[玩家可读的世界舞台产品设计](player-readable-world-stage.design.md)。
- 生命周期：`active`；Owner role：`producer_system_designer`。
- Last reviewed：2026-10-16。
- 专业域权威：[语义定位](../../world-simulator/viewer/viewer-pixel-world-semantic-positioning.prd.md)、[Fragment LOD](../../world-simulator/viewer/viewer-pixel-world-fragment-lod.prd.md)、[玩家可读渲染](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md)、[大世界可视化系统设计](../../world-simulator/viewer/external-agent-world-view.design.md)。

本文定义玩家观察世界时应读懂什么、怎样保持上下文与因果，以及哪些能力必须分别验收。具体 DTO、空间算法、渲染管线、平台依赖、容量数值及测试工具属于专业文档；本文不是当前发布报告。

## 设计适用性与生命周期闭合

- 设计判定：`paired-design`。
- 配对设计承接玩家旅程、信息层级、交互状态、来源提示、失败恢复与可访问体验，不另建要求真值。
- 当前实现、目标设计、运行验收和公开支持声明分别判断。历史 2D overview/zoom 已退役，设计方向和旧截图不能证明现行能力。

## 1. 目标

让玩家在世界现场读懂当前目的、相关 Agent/地点/路线、关键阻塞、可采取的下一步，以及已接受行动造成的权威世界反馈。空间舞台采用长期的 2D top-down / orthographic 像素工业指挥棋盘方向，作为默认 primary decision surface；目标、command、receipt、selection 上下文相邻且可展开，不以永久多栏挤压世界，也不把命令路径藏到不可发现。

没有已接受的玩家行动或可归因世界后果时，必须明确没有相应进展。世界繁忙、Agent 自主活动、动画、renderer 正常、诊断可见都不是玩家成功。

产品成功分为三个独立结果：玩家能找到并理解相关对象；能通过合法高层指导影响自己有权控制的 Agent；能辨认原行动的结果并继续或恢复。观察成功不等于指导成功，指导被接受不等于世界效果已经发生。

## 2. 首读层级与下一步

- 优先呈现目的、相关行动者/路线、关键 blocker 和下一决策；世界背景支持理解但不压过行动与因果。
- 结果能关联原目标，并有继续、纠正、中断或恢复入口。主要 command 从舞台或其紧邻上下文一步可发现；不可用时就近解释原因。
- 相关对象、选中/聚焦、父级或关系上下文可辨。不存在相关对象时显示为空/不可用，不以默认选择制造目标或控制资格。
- 降低密度先隐藏次级标签、细节及装饰，不无差别缩小关键对象或文本；视觉强调不产生新的交互资格。
- 布局、控件、文案、渲染层次与可访问性实现由 Viewer/视觉 authority 决定；Shell 的稳定方向不因本文补充而重排。

### 2.1 P0 冻结与 P1-A Agent Context Lite 边界

#### P0 Shell Lite：冻结产品方向

保持世界优先、单一 Next Move 主动作、从属 Objective/Player Leverage、单一因果回执及按需展开的上下文。冻结不是实现完成或公开发行结论。允许来源时效、权限、因果唯一性、窄屏和恢复的 focused 修正，不以常驻三栏、底部 hotbar 或大范围 HUD 重排作为前置。

#### P1 有序 rollout 与阶段依赖

1. **P1-A Agent Context Lite**：一个当前选中 Agent 的身份、状态、来源/时效、适用目标、下一步、blocker、Activity、Intent、Action Receipt 与权限。
2. **P1-B World semantic presentation**：在 P1-A 语义成立后扩展有权威身份、关系种类/状态、路线或位置来源的世界表达；邻近、同屏、文本不能补齐关系，不扩展 Agent 控制权。
3. **P1-C Major World Event**：事件具有身份、种类/严重度、来源、因果锚点、时效、生命周期和重放/重组语义后，再设计注意力。事件不是 Action Receipt，不建立第二条玩家因果链。

当前专业实现收据记载 P1-B snapshot 派生 stage cues 和 P1-C crisis-only additive projection/ambient feed；crisis 没有空间 anchor，因而没有 stage marker/highlight。生产 Major Event 只消费显式 runtime/operator audience policy，默认 `Unknown` fail closed，明确 `Public`/`Restricted` 才披露。登录、选择、控制或 Director 权限都不产生该 policy。局部实现收据不证明完整 P1 或公开发行。

#### P1-A：Agent-only Context Lite

选中只建立解释上下文，不等于绑定、认领或可控制；授权沿用[Agent 对话与 Prompt 控制](agent-conversation-and-prompt-control.prd.md)和[Agent 自治、委托与责任连续性](agent-authority-ownership-and-accountability.prd.md)。

只组合已经发布且明确适用于该 Agent 的可读身份、公开且有来源的所在/父级上下文与状态。Objective、Next Move、blocker、Player Leverage、执行状态只有明确绑定该 Agent 才可作为其上下文；global summary/primary intent 仍属于玩家，不能重命名为 Agent 的私有目标。

Activity、权威 Intent、玩家 `local_pending` 草稿、Action Receipt 与通信反馈独立呈现。Intent 需要 Agent/目标匹配及明确接受/阻断语义；草稿不是 Agent 当前 Intent。回执只引用现有 Player/Cinematic 单一因果锚点；没有绑定回执时保持无回执，不从 Feed、距离、同屏、文本、客户端时间或 raw object 推导成功、关系、ownership、计划、资源或能力。

Capability 需要适用主体、明确来源、权限、时效和恢复路径。缺失时不可用/待同步或省略，不用全局推荐或其他对象的能力填充。Facility、Territory、Organization、Depot、Module、Location 不继承 Agent 的语义，等待各自权威合同。

#### P1-A 验收映射

| ID | 产品承诺 | 组合验收 |
| --- | --- | --- |
| P1A-CTX-01 | 从世界/目标选中 Agent 后，0–1 次情境转换内辨认身份、适用状态、目的和下一决策；缺字段明确为空。 | RW-1、RW-6 |
| P1A-CTX-02 | 当前确认、最近已知、冲突、未知或失效不被抹平；影响行动时有刷新、等待、改道或停止路径。 | RW-8、RW-9 |
| P1A-CTX-03 | Activity、Intent、草稿、单一因果回执与非因果反馈独立；选择切换不改写原行动归属。 | RW-3、RW-4 |
| P1A-CTX-04 | capability 只来自明确适用的权威投影与权限；global summary、选中/可见/邻近不创造控制权。 | RW-2、RW-6 |
| P1A-CTX-05 | 世界优先、诊断次级；目的、阻塞、回执和恢复在窄屏、CJK、辅助技术下仍可理解。 | RW-1、RW-4、RW-5 |

#### P1-A 明确非目标

不重做 P0 Shell；不建立全实体通用 Inspector；不一次迁移 Market、War、Governance、Depot 或完整 Programs/Protocols；不增加 global inventory、capability hotbar、直接移动/攻击/采集；不改变 runtime/protocol、经济、治理、ownership 或数值规则；不把 plan/rationale/ETA/memory/provider/prompt/auth 诊断变成默认玩家详情。

<a id="stage-subject-identity"></a>
### 2.2 观察焦点、行动对象与回执归属

本节细化 REQ-AGENT-STAGE-001/005，不新增控制方式。

| 语义 | 合法来源与行为 | 禁止的替代 |
| --- | --- | --- |
| 观察焦点 | 玩家选择、授权搜索定位；为了初始构图可采用明确为展示用途的默认视点。 | 把列表首项、默认视点或可见对象称为玩家已控制对象。 |
| 行动对象 | 当前可用的权威动作/指导上下文，以及经过适用授权校验的明确对象选择。提交后属于原操作。 | 仅凭默认焦点、推荐变化或列表顺序补齐、替换提交目标。 |
| 回执归属 | 原操作、Agent/任务/目标及其权威结果的明确关联；无主体的玩家级结果保留玩家级语义。 | 回执回来时借用当前 selection、首个 Agent、最新推荐或文本猜测主体。 |

玩家提交 A 的指导后查看 B，A 的延迟结果仍属于 A；选择 B 不能把该结果改成 B 的成功，也不能自动重发动作。B 的上下文没有绑定结果时明确无回执，玩家级原回执仍可追溯。多条历史记录可以查询，但当前因果解释仍引用同一权威操作，不复制成多次效果。

未取得明确行动对象时，可保留只读浏览或未提交草稿，并提供选定合法对象/重新同步入口；不能为了让按钮可点击而静默借用 fallback 对象。观察权撤销时，应删除不再获准披露的细节，而非把它们作为最近已知保留。

## 3. 空间关系与来源

- 不承诺所有对象都有精确坐标。派生/抽象关系位置必须与权威世界位置可区分，不能用于宣称真实距离、可执行路线、资源地点或控制资格。
- marker、halo、标签或 overview 可为可读性放大，但表现尺寸不是物理尺寸；有权限的权威文本/数值优先。
- 关系线、箭头、热点只解释已支持的关系或效果，不从邻近、同屏或装饰产生关系、因果、所有权或动作入口。
- terrain/fragment blocks 只提供空间和材料语境，细节增加不产生 hover/selection、edit/harvest/build 能力。
- 多尺度呈现保留目标、行动者、关键路线、blocker、下一步和返回路径；不是恢复退役 2D/zoom 功能的交付声明。

### 3.0 产业关系与空间层级的持续要求

权威数据支持时，应识别工厂/配方/产品/物流节点，区分物料/电力/数据流向，以及有来源的吞吐、损耗、瓶颈、拥塞与告警；根因线索能定位 location/Agent/module。分层档案支持时，符号区分 R1~R5 和 bootstrap/scale/governance，状态独立且不只靠颜色。

World/Region/Node 分别突出热区/主干流、区域关系、配方/库存细节。缺数据显示未知/不可用，推断根因与确认事实分开。当前 routes、hotspots、HUD 不证明完整产业图谱、吞吐流、分层符号或三档 semantic zoom 已交付。

### 3.1 观察范围、时效与未知边界

动态对象、位置、路线、容量、部署和状态区分当前授权范围内确认、仍可披露但可能变化的最近已知，以及未取得/失效/冲突。公共已结算结果是历史事实，不自动授予实时经营情报、精确位置或控制权；同样不能用动态观察改写已结算历史。

范围外、未侦察、加载失败、缓存过期、组织归属、默认选择或界面可达都不意味着当前全知。位置的几何来源与信息时效是两个独立问题：曾经权威的坐标可以成为最近已知，派生位置也不能因刚刷新就变成精确坐标。

影响当前行动的不确定信息提供适用的刷新、等待、改道、停止或返回安全入口。专业域决定可见性、TTL、授权、DTO；玩家与 Agent 不要求共享相同原始 observation。

#### 3.1.1 来源冲突、刷新竞态与行动降级

- 冲突先被保留和解释，不以客户端时间、地图排序或最后到达的响应选出真值。多个当前动态观察互相矛盾时显示待重新确认。
- 刷新中旧信息最多是仍被授权披露的最近已知；旧请求晚到不覆盖新的已确认状态。重复刷新、重连、跨入口不产生第二份观察资格或世界效果。
- 只读浏览可以保留来源说明；没有副作用的方案预览必须标注不确定性。会影响资源、权限、路线承诺、设施状态或他人权利的生效操作，必须由对应权威在实际接受/生效边界重新确认，或明确拒绝/要求重新预览。
- 保存未提交草稿不等于激活新目标。预览时有效不保证提交时有效；上下文改变后不自动批准改写后的方案。
- Agent 可以提出待核验建议，不得把缓存/推断/冲突一方表述为确认事实并据此自动提交高后果行动。
- 已授权的 stop、撤销及结果回查遵守各自控制合同，不因渲染器失败或观察陈旧被无差别禁用；它们也不能越过服务器授权。
- 反馈说明原因、仍可信内容、受阻操作和恢复入口。结果不确定时先回查原操作，不以新键重做。专业 trace 能对账显示分类、重新校验和最终接受/拒绝原因。

### 3.2 可探索的大世界与网页/桌面客户端

<a id="world-exploration-surfaces"></a>
长期旅程是世界概览 → 区域/Fragment → 授权搜索/定位 → 对象检查 → 权威结果 → 合法指导 → 返回总览/当前目标，而非只看 Agent 面板或日志。

- 有平移/缩放或等价尺度切换、明确返回总览/目标，不靠无边界拖动寻找对象。
- 搜索、选择、检查和合法跟随只覆盖获准且有数据的 Agent/地点/设施/路线/事件。缺事件空间 anchor 时在 Feed 检查，不伪造 marker。
- 高密度、聚合、过滤、部分加载、视点裁剪与对象被权威删除含义不同；对象暂时不在画面内不等于世界中不存在。
- 重叠对象可经可访问的候选列表/等价入口可靠选择。自动跟随可退出；手动浏览不被持续抢回视点。来源或权限失效时停止自动追踪并给出合法恢复路径，不默认切换到另一个 Agent。
- 平移、选择、跟随、过滤、图层和下钻只是观察。自己的合法 Agent 可进入已有目标/Prompt/委托；地图点击不直接采集、建造、移动。
- 正式 Web 和受支持桌面原生客户端分别在自身界面交互。桌面可以内嵌同一 Web 舞台，但只打开系统浏览器、静态图或 Launcher 按钮不算客户端内可视化。
- 断连、局部加载失败、renderer 启动失败或运行中失效时，保留仍被授权披露的历史结果、目标、可恢复选择及可访问文本；明确画面是否陈旧，提供刷新/返回，不能把空画布解释成空世界。
- 移动/触控、键盘、CJK 和低高度下保持同一概念路径。具体支持的平台和质量预算由专业权威逐组合验证，不从 Web 成功推导其他端成功。

## 4. 玩家因果与诊断边界

只有已接受意图及其可归因世界后果能够表达玩家影响。accepted/submitted/queued 不是完成；完成但无进展与真实进展不同；阻断、拒绝和结果待确认各有恢复入口。世界重放不表达成新收益，重组后的结果按权威状态重新解释，不用旧动画保留成功。

目的、blocker、回执和恢复路径有可访问文本或等价语义。renderer、runtime source、原始 DTO、性能、测试控制按需访问且次级；文本降级不另建世界渲染或世界真值。mock、DOM、软件 raster、GPU 截图与真实运行结果分别证明各自范围。

## 5. 叶级产品要求与验收

下面五项继续使用原 Trace ID。新增细则是已有义务的展开，不另发重复 ID。核对基线为 `a2389f437e6cedc3b870a91d33a1cdf40a00a8da`；日期为 2026-10-16。设计状态表示本修订目标合同的覆盖范围，不表示代码或验收通过。实际提交、独立评审与运行证据由对应 PR/Task 承载。

<a id="req-agent-stage-001"></a>
### REQ-AGENT-STAGE-001：世界舞台必须先呈现目的与因果

Trace ID: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616

- 功能点：首读目标、对象、blocker、下一步及原行动结果；观察焦点不补行动/回执归属。
- 系统设计状态：已覆盖（目标合同）；依据：[共享舞台](../../world-simulator/viewer/external-agent-world-view.design.md#view-scene)、[主体绑定](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.design.md#subject-binding)、[质量预算](../../world-simulator/viewer/external-agent-world-view.design.md#view-quality)。
- 实现状态：未核实；剩余：现有 fallback 主体路径需按新合同修复并做同候选全链验收，文档不代替代码整改。
- 验收：AC-AGENT-STAGE-001；核对日期与基线沿本节。

<a id="ac-agent-stage-001"></a>
### AC-AGENT-STAGE-001：玩家能从舞台读到下一决策

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616

代表性目标、阻塞和结果在密度变化后仍可读；能到达继续/纠正/中断/恢复。A 的操作在途时切换 B、调整过滤或列表重排，结果仍关联 A 的原操作，不制造 B 的进展。无行动/回执、玩家级无 Agent 的结果及仅环境活动均不补造主体。按[首读测试](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-readability)判定理解与误归因，不以截图存在判通过。

<a id="req-agent-stage-002"></a>
### REQ-AGENT-STAGE-002：动态观察必须保留来源、时效与未知

Trace ID: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3

- 功能点：当前/最近已知/未知/冲突、来源范围、提交时重新确认和恢复。
- 系统设计状态：已覆盖（目标合同）；依据：[空间查询与同步](../../world-simulator/viewer/external-agent-world-view.design.md#view-query)、[观察到提交](../../world-simulator/viewer/external-agent-world-view.design.md#view-admission)、[来源适配](../../world-simulator/viewer/external-agent-world-view.design.md#view-adapter)。
- 实现状态：未核实；剩余：真实权威 context/控制版本接线、竞争与故障注入、客户端恢复验收。
- 验收：AC-AGENT-STAGE-002；核对日期与基线沿本节。

<a id="ac-agent-stage-002"></a>
### AC-AGENT-STAGE-002：陈旧或冲突观察不会代签真值

Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3

旧响应晚到、冲突、刷新期间撤权、提交前过期/转让或目标 revision 改变时，不覆盖新事实，不先绘制私有内容再鉴权。低后果预览带不确定性，高后果操作由权威重验或阻断；stop/原操作回查按自己的授权可达。结果不确定不盲重做。按[时效与提交测试](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-freshness)对账原操作及原因。

<a id="req-agent-stage-003"></a>
### REQ-AGENT-STAGE-003：大世界必须可多尺度探索

Trace ID: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf

- 功能点：授权范围内总览/区域/Fragment/对象浏览、平移/缩放、搜索、选择/跟随、返回目标和高密度可用性。
- 系统设计状态：已覆盖（目标合同）；依据：[查询](../../world-simulator/viewer/external-agent-world-view.design.md#view-query)、[部分数据与适配](../../world-simulator/viewer/external-agent-world-view.design.md#view-adapter)、[容量与质量](../../world-simulator/viewer/external-agent-world-view.design.md#view-quality)。Fragment LOD 仅承接背景细节降级，不代替整个探索合同。
- 实现状态：未核实；剩余：空间投影、分页/聚合、交互、性能及真实浏览器验收。
- 验收：AC-AGENT-STAGE-003；核对日期与基线沿本节。

<a id="ac-agent-stage-003"></a>
### AC-AGENT-STAGE-003：世界概览至对象再回到当前任务

Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf

给定多区域、部分加载、无精确坐标和重叠对象；浏览、定位、选择、跟随/退出及返回后，身份、空间层级、来源、blocker 和主目标连续。分页不被误作全量删除，聚合/过滤不制造对象不存在。稀疏/密集、长文本及支持输入方式分别按[探索](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-exploration)和[容量](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-performance)验收。

<a id="req-agent-stage-004"></a>
### REQ-AGENT-STAGE-004：网页和原生桌面客户端都要能实际探索世界

Trace ID: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a

- 功能点：被声明支持的 Web/原生端各自在界面内浏览、定位、检查、重连和图形恢复。
- 系统设计状态：已覆盖（目标合同）；依据：[原生容器](../../world-simulator/viewer/external-agent-world-view.design.md#view-native)、[降级与恢复](../../world-simulator/viewer/external-agent-world-view.design.md#view-recovery)、[逐端验收](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-platform)。
- 实现状态：未核实；剩余：实际 WebView 依赖锁定、各 OS 发行资产和真实恢复证据；目标平台表不是当前支持声明。
- 验收：AC-AGENT-STAGE-004；核对日期与基线沿本节。

<a id="ac-agent-stage-004"></a>
### AC-AGENT-STAGE-004：两个 surface 分别有可交互世界证据

Trace refs: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a

同一真实世界、Agent 成果和固定候选，分别在 Web/受支持原生包内缩放、定位、检查结果、断连回访，并覆盖启动失败与 ready 后图形失效。结果同义且权限不扩大；系统浏览器跳转、静态图、另一平台通过不代签。

<a id="req-agent-stage-005"></a>
### REQ-AGENT-STAGE-005：地图只能进入合法间接指导

Trace ID: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096

- 功能点：有权 Agent 的目标/Prompt/委托与原操作回执衔接，观察不产生世界副作用。
- 系统设计状态：已覆盖（目标合同）；依据：[指导边界](../../world-simulator/viewer/external-agent-world-view.design.md#view-guidance)、[提交重验](../../world-simulator/viewer/external-agent-world-view.design.md#view-admission)、[主体绑定](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.design.md#subject-binding)。
- 实现状态：未核实；剩余：服务器重验、原操作幂等回查、无权/转让/主体切换负例及真实结果。
- 验收：AC-AGENT-STAGE-005；核对日期与基线沿本节。

<a id="ac-agent-stage-005"></a>
### AC-AGENT-STAGE-005：选中对象不授予世界控制

Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096

自己的 Agent、可见但无权的 Agent、地形和 pending/committed 操作并存时，浏览/过滤/选择不提交世界 action；合法指导的保存、接受、生效与后续世界结果不同。转让、撤权、旧草稿、重复提交或断连恢复不能改变原主体或产生第二次效果。验证入口：[权限与归属](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-identity)。

## 5.1 叶级 owner、authority、evidence 与 test tier 追踪

| REQ / AC | 专业 owner 与边界 | 专业权威 | 验证证据与层级 |
| --- | --- | --- | --- |
| 001 | product/visual 定义首读；Viewer 投影；Runtime/gameplay 拥有因果；QA 判定理解。 | [玩家可读渲染](../../world-simulator/viewer/viewer-pixel-world-player-readable-rendering.prd.md#41-commercial-surface-dto) | PW-IDENTITY/PW-READABILITY；确定性 `test_tier_required`，真实因果/理解 `test_tier_full`。 |
| 002 | Runtime/Agent 校验观察与生效；Viewer 表达；QA 注入竞态。 | [观察到提交](../../world-simulator/viewer/external-agent-world-view.design.md#view-admission) | PW-FRESHNESS；本地确定性与 `test_tier_full` 实际组合分别留证。 |
| 003 | Viewer/visual 的空间/密度/交互；Runtime 授权数据；QA 容量。 | [空间适配](../../world-simulator/viewer/external-agent-world-view.design.md#view-adapter) | PW-EXPLORATION/PW-PERFORMANCE；完整范围 `test_tier_full`。 |
| 004 | Viewer/Launcher 容器、发行资产；QA 各平台验证。 | [Native](../../world-simulator/viewer/external-agent-world-view.design.md#view-native) | PW-RECOVERY/PW-PLATFORM；`test_tier_full`，不相互代签。 |
| 005 | Agent/Runtime 指导、权限和结果；Viewer 不另建写路径。 | [指导](../../world-simulator/viewer/external-agent-world-view.design.md#view-guidance) | PW-IDENTITY/PW-FRESHNESS；权限合同 required，真实结果 full。 |

## 5.2 组合验收

- RW-1：不读诊断即可识别世界、目标、相关对象/路线、blocker、下一步。
- RW-2：空间关系可读；派生/抽象位置不冒充精度、资源事实或能力。
- RW-3：区分原玩家行动的可归因结果与环境/自治活动，并可继续、纠正、中断或恢复。
- RW-4：目的、行动、反馈优先，诊断次级且可达。
- RW-5：证据关联当前 gameplay/runtime/Agent/Viewer/QA 候选，单图/mock/fallback 不单独成立产品结论。
- RW-6：舞台 primary read 与相邻上下文连续；command 可发现，不以详情取代世界。
- RW-7：密度变化先退次级标签；terrain 非 edit/harvest/build affordance。
- RW-8：公共结果、当前确认、最近已知、未知/失效分开；非当前信息有恢复路径。
- RW-9：冲突/旧响应/提交前失效不代签真值；不确定预览与高后果重验分开。
- RW-10：有权总览至区域/对象/事件再返回目标，聚合/未加载不伪装全知或空世界。
- RW-11：Web 与受支持原生各自实际交互，不由浏览器跳转或截图代签。
- RW-12：观察衔接合法间接指导和原操作回执，不直接操控世界。

### 5.2.1 验收权威与证据边界

具体场景、指标、视口、数据密度、输入与平台组合见[像素世界验证手册](../../testing/manual/pixel-world-visualization-validation.manual.md)。普通 CI 先覆盖可确定复现的合同，不强制每个文档 PR 跑付费模型、GPU 或真人测试。真实玩法、S6、L5、发行与公开声明仍各按原专业权威；缺证据写未验证，不降低标准制造通过。

## 6. 范围

覆盖首读、对象/关系、比例/来源、因果、观察式探索、两端同义、支持文本和故障恢复。具体 renderer、资产、世界规则及写接口不在本产品分册定义。

## 7. 接口 / 数据

阅读语义为 `玩家目的 → 相关对象/关系 → 来源与适用性 → 原行动结果 → 下一步`。产品不冻结 DTO/坐标/TTL/相机/缓存字段；专业适配必须保留这些语义而非只保留绘图坐标。

## 8. 里程碑

保持 P0 与 P1-A/B/C 依赖。实施先统一主体和来源，再接有界读模型/探索，最后扩大原生与容量声明；每项以对应实现和验证通过为准，不按文档合入宣称阶段完成。

## 9. 风险

主要风险是默认焦点污染行动/回执、部分数据被误作全量、权限缩小时泄漏旧详情、性能降级丢失关键对象，以及把目标架构/旧截图当当前交付。专业合同必须为这些风险提供负例。

## 10. 非目标与专业边界

不新增直接世界动作、玩法控制权、资源事实或发布等级；不恢复第二 Viewer、旧 3D 或旧 2D 工具链；不冻结 `commercial_surface`、`fragment_terrain`、`position_source`、shader、LOD 阈值、颜色、资产及性能数值。产业图谱、吞吐/队列、设施流、动画、原生/semantic zoom 的目标不等于当前存在。2026-05-28 brainstorm 仅为候选输入，不是当前 authority。

## 11. 未决问题与假设

运行实现、平台依赖锁定、真实可读性及容量结果仍待相应实现/验收；设计覆盖不消除这些交付缺口。缺少当前授权、位置、关系、因果或时效时保持未知/待确认并提供合法恢复，不用视觉邻近、列表首项或缓存填补事实。
