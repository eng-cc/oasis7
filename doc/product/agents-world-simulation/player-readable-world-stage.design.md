# 玩家可读的世界舞台产品设计

## 文档身份

- 配对产品 PRD：[玩家可读的世界舞台](player-readable-world-stage.prd.md)；上位产品 PRD：[`prd.md`](prd.md)。
- 生命周期：`active`；Owner role：`producer_system_designer`；Last reviewed：2026-10-16。
- 专业 authority：[Viewer 手册](../../world-simulator/viewer/viewer-manual.manual.md)、[页面模块设计](../../world-simulator/viewer/viewer-page-module-design-2026-06-18.design.md)、[视觉规范](../../world-simulator/viewer/viewer-visual-design-spec-2026-06-05.design.md)、[大世界系统设计](../../world-simulator/viewer/external-agent-world-view.design.md)。
- 结构裁剪：本设计按阅读顺序、对象、空间、反馈与恢复组织产品体验，不使用技术十二段骨架；目标见 §1，状态/交互见 §2–4，验收见末表。具体布局、DTO、renderer、平台和性能回到专业 authority。

本设计描述可验收的目标体验，不把未来地图、多尺度、原生世界窗口或截图写成已交付。现有 Shell 的世界优先方向和 Player/Director 边界不变。

## 1. 阅读顺序

玩家应先辨认当前世界与目的、相关行动者/路线、主要 blocker 和下一决策，再看原行动结果及支持信息，最后按需读诊断。命令路径一步可发现，但不以三张平级卡片、常驻多栏或长日志压过世界。

首次进入、回访和故障恢复共用同一概念结构：世界舞台是主面，目标/下一步是紧邻上下文，选中详情按需展开，Action Receipt 解释原操作，World Feed 只是环境活动。没有目标、没有可见对象、没有回执都是有解释的状态，不填充“看起来完整”的假数据。

## 1.1 P0 冻结与 P1-A Agent Context Lite

### P0：稳定框架，不扩大常驻界面

保持单一世界舞台、Next Move 主区、从属 Objective/Player Leverage、按需 Targets/Command 和次级 Diagnostics。允许来源、授权、回执唯一性、窄屏与恢复的 focused 修正，不借本次设计重排 Shell。

### P1 有序组合

P1-A 先确立一个选中 Agent 的身份、来源/时效、适用目标、Activity、Intent、回执与权限；P1-B 再加入具有独立权威的对象、关系/路线、位置；P1-C 最后处理具有身份、来源、时效和生命周期的重大事件注意力。后一阶段不能用未成立的前置代替验收。

专业实现收据已有 P1-B stage cues 和 P1-C crisis-only ambient projection；crisis 缺空间锚点时没有 marker/highlight。Major Event 消费独立显式 runtime/operator audience policy，默认 Unknown 不披露；登录、控制、selection、Director 不产生披露资格。局部实现不代签全部阶段。

### P1-A：只从选中的 Agent 开始

| 阅读步骤 | 应呈现的内容 | 缺失/不适用时 |
| --- | --- | --- |
| 辨认 | Agent 可读身份、公开且有来源的所在/父级上下文和状态。 | 明确未知/不可用，不挑另一 Agent 替代。 |
| 理解局势 | 仅明确绑定该 Agent 的 Objective、Next Move、blocker、Player Leverage。 | 全局玩家目标保留全局标签，不重命名成私有目标。 |
| 区分运行 | Activity、权威 Intent、未提交草稿各自状态。 | local_pending 不是已接受 Intent，通信成功不是世界执行成功。 |
| 确认结果 | 原 Player/Cinematic 单一因果锚点中确实属于该主体的结果。 | 无绑定回执；非因果反馈独立，不补效果/成功。 |
| 继续/恢复 | 具有适用来源、权限、时效的 capability 或下一入口。 | 说明待同步/无权/不适用；可只读，不制造按钮可用性。 |

从选中到辨认相关上下文保持 0–1 次情境转换。选中/可见不等于认领或控制；账号和授权沿[Agent 对话与 Prompt 控制](agent-conversation-and-prompt-control.prd.md)与[自治、委托与责任连续性](agent-authority-ownership-and-accountability.prd.md)。不从距离、同屏、Feed、名称、raw object 推断计划、关系、资源、成功或权限。

### P1-A 组合验收映射

| 设计检查 | 可观察承诺 | 产品验收 |
| --- | --- | --- |
| Agent identity/context | 正确主体、适用状态和空状态，不制造默认目标。 | P1A-CTX-01 / RW-1 / RW-6 |
| Source and freshness | 当前、最近已知、冲突、未知及恢复方向可辨。 | P1A-CTX-02 / RW-8 / RW-9 |
| Activity/Intent/Receipt | 三者与草稿、通信反馈不合并；切换对象不改归属。 | P1A-CTX-03 / RW-3 / RW-4 |
| Capability boundary | 只有明确适用的权威能力；全局信息不改变控制。 | P1A-CTX-04 / RW-2 / RW-6 |
| World-first readability | 解释层次级，目的/blocker/下一步可读。 | P1A-CTX-05 / RW-1 / RW-4 / RW-5 |

### P1-A 非承诺

不建立所有实体的通用 Inspector；Facility/Territory/Organization/Depot/Module/Location 不继承 Agent 控制或私有目标；不新增 hotbar、inventory、经济/治理数值、直接动作；不把 plan/rationale/ETA/memory/provider/prompt/auth 诊断作为默认详情；不在此定义新的 Major Event 因果面、DTO、坐标、颜色或布局断点。

## 2. 对象辨认与聚焦

可见对象可以观察，有权对象才可能被指导，当前行动对象与当前选中对象可以不同。初始构图的默认视点必须是展示行为，不暗示认领、控制或已有目标。

<a id="subject-continuity"></a>
### 2.1 主体连续性

玩家对 A 提交指导，再选择 B：舞台和详情进入 B 的观察上下文，原操作继续关联 A。A 的回执回来时，在现有回执位置说明 A/原目标及结果，不重新把视点拉回 A，也不污染 B 的私有结果。可提供“查看原对象/原操作”，但只有仍获准披露时可进入；撤权后删除受限内容而非显示旧详情。

无 Agent 绑定的玩家级结果保持玩家级；同一结果在详情/回执引用不是两次效果。推荐对象改变、过滤、列表重排、重连和页面重载都不改变已提交对象。缺明确行动对象时，Next Move 引导合法选择或同步，不静默借用首项。

### 2.2 对象能力矩阵

| 对象/状态 | 可提供的观察 | 指导边界 |
| --- | --- | --- |
| 自己当前有权 Agent | 适用身份、来源、活动、目标及原回执。 | 只进入现有合法高层目标/Prompt/委托/审批/stop。 |
| 可见但不可控 Agent | 只读、已授权公开的内容。 | 明确只读，不复用另一个 Agent 的能力。 |
| Location/设施/Depot/路线 | 该实体 authority 已提供的身份、状态、位置/关系与来源。 | 不自动套 Agent 模板；地图点击无采集/建造/移动。 |
| terrain/Fragment block | 世界背景及允许的材料语境。 | 非交互背景不显示可点击手势或控制暗示。 |
| 有权威 anchor 的世界事件 | 事件自身身份、来源、时效及检查入口。 | 环境提示不是玩家因果回执。 |
| 无 anchor 的事件 | 相邻 Feed 的可访问事件内容。 | 不放置估算 marker/highlight。 |
| 聚合符号/重叠对象 | 已授权成员的下钻或候选列表；说明聚合范围/部分性。 | 聚合不是新实体，不批量授予操作权。 |

### 2.3 选择、遮挡与消失

选中反馈同时有形状/文本/焦点等信号，不仅依赖颜色或动画。重叠时给稳定候选列表或等价选择方式，不靠重复像素试点。tooltip/选中标签避让主要操作和文本，无法放置则转入相邻详情，不让画布截断唯一信息。

对象被过滤、离开视点、所在区域未加载、来源失效、权限撤销和权威删除分别解释。过滤当前选中对象时保留“已被过滤”的上下文与清除过滤入口；跟随失去可信位置时暂停追踪，不选另一个对象。撤权时立即清除不可披露内容；可保留的只是经授权的通用不可用提示。

## 3. 空间与比例诚实

权威几何、派生关系布局、最近已知和未知必须可区分。几何来源与新鲜度独立：刚收到的派生坐标不是精确位置；旧的权威坐标不是实时位置。放大 marker/halo 不表示真实尺寸，关系线不创造关系或可执行距离。

高密度优先保留当前目标、选中对象、关键 blocker 和相关路线；次级标签渐退。overview/detail 改变信息密度但保持身份、目的和下一步，不把画面裁剪当世界数据消失。未知不是零库存、空地图或不存在。

### 3.1 大世界可操作视觉体验与两端同义

默认旅程为“概览 → 区域/Fragment → 授权搜索 → 定位/选择 → 检查 → 合法指导 → 原结果 → 返回目标”。这是未来可验收目标，不是当前 2D/zoom/native 已交付声明。

| 玩家操作 | 最小反馈 | 失败或中断 |
| --- | --- | --- |
| 平移/缩放/下钻 | 当前范围、尺度、选中身份、目标及返回入口。 | 旧范围可标最近已知；新范围加载失败不是空世界。 |
| 搜索 | 明确当前列表过滤还是全授权范围搜索，显示加载/无结果/部分结果。 | 旧查询不覆盖新查询，不泄漏未授权对象数量。 |
| 定位 | 有可信位置才定位；关系位置明确抽象。 | 已知对象暂不能定位时，仍能读获准详情并刷新/返回。 |
| 跟随 | 明确正在跟随哪个对象，随时退出。 | 手动浏览退出跟随；位置失效暂停，恢复需明确重新跟随。 |
| 选择/检查 | 所选对象、相关状态、来源/时效与适用能力。 | 选择不执行，晚到详情不覆盖新对象。 |
| 返回总览/当前目标 | 两个意图可区分，保留合法目标上下文。 | 目标不可定位时返回其文本上下文，不跳到任意对象。 |
| 提交指导 | 方案、主体、适用授权及保存/接受/生效/结果区别。 | 版本冲突/观察过期先解释与重验，不自动批准变化方案。 |

Web 与原生端分别在自己的界面完成，原生可复用 Web 实现，但外部浏览器不能代签。

<a id="exploration-transitions"></a>
### 3.2 关键状态转换

| 当前状态 | 触发 | 下一状态与保留内容 |
| --- | --- | --- |
| 初始同步 | 获得授权的首份一致读模型 | 可浏览；明确完整/部分/未知，无默认控制对象。 |
| 自由浏览 | 选择授权对象 | 选择上下文打开，目标/原回执不改，camera 不必自动跳转。 |
| 已选择 | 明确开始跟随 | 跟随该身份；仅可信位置更新可移动视点。 |
| 跟随中 | 手动平移/尺度浏览或退出跟随 | 自由浏览，保留选中身份；后台更新不抢回视点。 |
| 跟随中 | 位置失效/断连 | 跟随暂停，最近已知标记；不得推演当前位置。 |
| 任意浏览 | 详情/搜索旧响应晚到 | 丢弃不适用结果，不改变当前视点、选择或权限。 |
| 已选择 | 局部数据暂未加载/过滤 | 保留获准上下文并说明原因；有刷新/清过滤/返回。 |
| 任意状态 | 换账号/世界或撤权 | 清除不适用私有数据、交互和旧请求，再按新范围初始化。 |
| 提交等待 | 网络/renderer 失效 | 保留原操作身份、结果待确认及回查入口，不重投。 |

drawer/Focus/IME 的 Escape 顺序与焦点返回继续遵守[页面交互合同](../../world-simulator/viewer/viewer-page-module-design-2026-06-18.design.md#27-interaction-state-machine-and-focus-contract)：先最内层 drawer，再 Focus；输入法合成中不截获。跟随退出提供独立可访问入口，不能抢占既有 Escape 语义。

### 3.3 输入方式与探索可达性

鼠标拖动和触控手势不是唯一操作。通过可聚焦的方向/缩放/尺度/返回控件或等价列表，单指针不拖拽与键盘也可到达相同授权对象。画布快捷键仅在画布上下文生效，不吞掉输入框、聊天、文本选择和 IME。

触控选择保留明确的 Selected 上下文，说明不只依赖 hover；tooltip 可显式关闭并返回触发点。支持的屏幕与输入方式在同一候选矩阵声明，未验证的组合不标支持。

## 4. 支持文本与诊断

目标、blocker、回执和恢复有文本/等价语义；原始 DTO、LLM 输入输出、模型、token、延迟和内部 source 默认次级。只有专业 surface 明确支持才展示复制/导出动作，导出不得夹带 token、私有观察或其他账号内容。

<a id="failure-experience"></a>
### 4.1 故障与恢复体验

| 情况 | 玩家解释 | 可用下一步 |
| --- | --- | --- |
| 初次没有 snapshot | 正在同步或尚无授权世界数据。 | 等待、刷新、返回；不是世界为空。 |
| 查询部分完成 | 仅当前已加载/获准范围，部分对象待加载。 | 重试本范围/继续浏览，不清空全世界。 |
| 陈旧或冲突 | 最近已知/信息冲突，说明对当前决策的影响。 | 刷新、等待、改道；高后果生效由权威重验。 |
| Renderer 不可用 | 图形原因与任务/网络状态分别说明。 | 文本/目标列表、原操作回查、合法控制和显式重试。 |
| ready 后图形失效 | 当前画面不再实时，不能继续命中旧像素。 | 重建图形会话；恢复后重新验证来源、选择和命中。 |
| 断连 | 新信息不可确认，既有获准历史结果仍可查。 | 重连/回查；不把已完成改成 pending 并重做。 |
| 权限撤销/换账号 | 当前内容不可继续披露。 | 清理旧内容后按新身份进入；不展示旧私有缓存。 |
| 原操作结果不确定 | 正在恢复确认，不写成普通失败或成功。 | 查询原操作，禁止以新键重复副作用。 |

文本降级不绘制第二套 JS 世界；故障恢复不自动提交指导、切换世界、增加权限或调用模型。未获准的内容不因“保留上下文”而继续展示。

## 5. 响应式与可访问性

CJK、长名称、低高度、窄屏和放大文本下保持主读、下一步与恢复。状态同时有文本/形状/位置等信号，颜色和运动不是唯一载体。减少动效时仍能辨认选中、阻塞和结果；装饰闪动不能夺取注意力。

键盘能够进入/离开画布和详情，有可见焦点和合理返回；不在每次数据刷新时抢焦点或播报所有实体更新。局部状态变化优先简短汇总，关键结果可读且不重复宣告成功。图形缺失时，文本入口仍能完成受支持的观察与安全返回。

密度、视口、DPR、语言、焦点、触控与低性能组合按[专项验证手册](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-accessibility)执行。产品不以一张桌面截图代替移动、辅助技术、实际 renderer 或真人理解。

## 6. 非承诺

不把 Locate Agent、overview、native、空间事件标记或具体手势当已实现；不新增世界动作、游戏规则、所有权或通用 Inspector；不定义厘米字段、LOD 阈值、shader、像素资产及帧率。实体/工业信息只在对应 authority 支持时展示，不用设计图补事实。

## PRD REQ/AC fragment mapping

| PRD requirement | PRD acceptance | 本设计承接 |
| --- | --- | --- |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[REQ-AGENT-STAGE-001](player-readable-world-stage.prd.md#req-agent-stage-001) | [AC-AGENT-STAGE-001](player-readable-world-stage.prd.md#ac-agent-stage-001) | 阅读顺序、主体连续性、原结果与反馈恢复。 |
| Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3；[REQ-AGENT-STAGE-002](player-readable-world-stage.prd.md#req-agent-stage-002) | [AC-AGENT-STAGE-002](player-readable-world-stage.prd.md#ac-agent-stage-002) | 来源/时效独立、冲突、提交重验与失败解释。 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[REQ-AGENT-STAGE-003](player-readable-world-stage.prd.md#req-agent-stage-003) | [AC-AGENT-STAGE-003](player-readable-world-stage.prd.md#ac-agent-stage-003) | 探索旅程、跟随、重叠、部分加载和多输入。 |
| Trace refs: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a；[REQ-AGENT-STAGE-004](player-readable-world-stage.prd.md#req-agent-stage-004) | [AC-AGENT-STAGE-004](player-readable-world-stage.prd.md#ac-agent-stage-004) | 两端自身界面交互、重连、图形与权限恢复。 |
| Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096；[REQ-AGENT-STAGE-005](player-readable-world-stage.prd.md#req-agent-stage-005) | [AC-AGENT-STAGE-005](player-readable-world-stage.prd.md#ac-agent-stage-005) | 观察/指导分离、主体绑定和无副作用浏览。 |

验收方法见[像素世界验证手册](../../testing/manual/pixel-world-visualization-validation.manual.md)，运行记录在实际 PR/Task。产品设计完成不代替系统实现、独立评审、真人理解或发行批准。
