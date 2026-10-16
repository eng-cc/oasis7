# 外部 Agent 大世界 Web 与桌面可视化系统设计

- 设计状态：`target contract`；当前实现和运行证据不得由本文推定。
- Canonical repository：`nil-studio/oasis7`；审读基线：`a2389f437e6cedc3b870a91d33a1cdf40a00a8da`。
- Owner role：`viewer_engineer`；共同审读：runtime_engineer、agent_engineer、game_visual_interaction_designer、qa_engineer、Launcher owner。
- 最后审读：2026-10-16；文档合同 revision：`2`。此为文档修订号，不是已发布 API major。
- 产品入口：[世界舞台](../../product/agents-world-simulation/player-readable-world-stage.prd.md)、[外部 Runtime 游玩](../../product/agents-world-simulation/external-agent-runtime-play.prd.md)、[玩家与 Runtime 协作](../../product/agents-world-simulation/player-runtime-collaboration.prd.md)。
- 依赖 authority：[Game API](../llm/external-agent-game-api.design.md)、[Runtime 协作](../../world-runtime/runtime/player-runtime-collaboration.design.md)、[当前 Viewer 手册](viewer-manual.manual.md)。

## 1. 问题、目标与非目标

让玩家在 Web 与桌面客户端内观察同一持久世界，从总览进入局部，理解 Agent/设施/路线/事件，连接合法高层指导并读取原行动结果。解决现有完整 snapshot 到 Rust render state 的局部链路与未来授权、分页、流式大世界读模型之间的适配问题。

目标是主体、权限、来源、时效、部分性、交互和恢复都可验证，不仅是“画出了对象”。非目标：另建世界引擎或第二 Viewer、直接地图采集/建造/移动、所有人全图可见、修改共识/世界 schema、自动调用模型或把所有实体套用 Agent 控制。

质量预算见 §9；这些是新切片的初始验收目标，未测量的结果不写成已达到。

## 2. 上游约束与相关角色

Runtime 拥有世界、控制版本、观察资格、行动 admission 和持久结果；API/身份 owner 提供授权查询、Context、撤销和原操作恢复；Viewer 只派生显示与本地交互；Launcher 提供实际受限原生容器；QA 分开验证数据合同、GPU、平台与真实世界。

### 2.1 需求承接与分配表

| 上游 requirement / product AC | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616；[AC-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-001) | 世界/目的/blocker/下一步与原行动结果可读，选择不改归属。 | [view-scene](#view-scene) | Viewer/visual；Runtime 因果投影。 | 不以局部 HUD 或静态图证明真实理解。 |
| Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3；[AC-AGENT-STAGE-002](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-002) | 来源/时效独立，冲突、旧响应、撤权和提交前失效有重验/阻断。 | [view-admission](#view-admission) | Runtime/API 的 Context、fence、授权与操作账本。 | Viewer 不自行裁决真值或新建 action admission。 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[AC-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-003) | 多尺度探索、部分数据、跟随/返回、高密度可用性。 | [view-query](#view-query)、[view-adapter](#view-adapter)、[view-quality](#view-quality) | Runtime 授权空间投影；Rust host/renderer；QA。 | 地形 LOD 不等于完整 semantic zoom；未验证无性能支持声明。 |
| Trace refs: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a；[AC-AGENT-STAGE-004](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-004) | Web/native 自身界面交互、重连和图形失效恢复。 | [view-native](#view-native)、[view-recovery](#view-recovery) | Launcher/WebView、发行资产、QA 逐 OS。 | Web 成功/外部浏览器不代签原生。 |
| Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096；[AC-AGENT-STAGE-005](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-005) | 只读选择进入合法高层指导，原操作至多一个效果。 | [view-guidance](#view-guidance) | owner API、Runtime task/goal/receipt。 | 不新增地图世界动作或模型权限。 |
| Trace refs: O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a；[AC-EXT-015](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-015) | 两个表面实际操作同一真实世界。 | [view-native](#view-native) | Viewer/Launcher/QA。 | 不证明两种真实 Runtime 或完整首局。 |
| Trace refs: O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf；[AC-EXT-016](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-016) | 从概览探索局部、搜索并返回目标，未知不伪造。 | [view-query](#view-query) | 授权索引及权威位置/关系。 | 不授予全图情报。 |
| Trace refs: O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096；[AC-EXT-017](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-017) | 世界现场连接合法高层指导与原回执。 | [view-guidance](#view-guidance) | Agent/Runtime 协作。 | 不把 accepted 或动画写成世界效果。 |

## 3. 当前状态、目标状态与差距

| 当前基线 | 目标适配 | 实现边界 |
| --- | --- | --- |
| canonical `viewer.html`、`viewer.js`、`oasis7_viewer_live`，software_safe 仅 alias。 | 共用同一 Viewer build 与应用状态。 | 不建立第二产品入口。 |
| `host_state.rs::build_render_state` 从有界 snapshot 输入派生显示状态；JS host 消费 Rust。 | 授权数据客户端先组装明确范围/版本的显示输入，再进入既有 Rust/renderer。 | 新 WorldView 查询不是现有完整 snapshot 的可无损替身。 |
| Renderer 已有按类型 reconcile、位置来源、地形 LOD、选中和宿主事件。 | 保留 ABI，补 partial/分页/裁剪的适配及来源侧带状态。 | 不把旧整场景重建问题重新当现状。 |
| 商业 HUD 的 active-agent fallback 可能进入 next-action/receipt target 派生。 | 观察焦点、原提交主体、原回执身份独立。 | 代码整改和负例仍须执行；文档不宣称已修复。 |
| 原生 Launcher/控制面本身不证明内嵌世界。 | 同一构建的受限 WebView 世界窗口。 | 当前平台支持和运行证据仍未由本文证明。 |

## 4. 边界与结构

<a id="view-scene"></a>
### 4.1 一个 Viewer、四层职责

```text
Runtime checkpoint / authorized events
  -> WorldProjectionReader (server: auth, cut, bounded spatial projection)
  -> WorldDataClient (client: page assembly, generations, cache, recovery)
  -> WorldSceneAdapter -> Rust host_state -> WASM renderer + accessible host UI
                     -> TaskContextPanel (independent authorized task/goal/receipt)
Owner guidance -> existing owner API -> Runtime control/admission -> original receipt
```

Viewer 不持有第二份资源账本、任务完成规则或 admission。外部 executor 私有 observation 不广播给 Viewer；同世界也必须按 audience 分投影。图形输入只含绘制所需、已授权字段；token、模型 Key、原始私有 prompt 和其他主体内容不进入 render DTO、DOM probe 或日志。

`observation focus`、`submitted subject`、`receipt subject` 是不同语义。当前 selection 只控制浏览。原操作保存 world/Agent/task/goal 与操作身份；回执按该身份关联，缺绑定不借用 active_agent_id。具体 HUD 合同见[主体绑定](viewer-pixel-world-player-readable-rendering.design.md#subject-binding)。

<a id="view-adapter"></a>
### 4.2 当前链路到 WorldView 的适配合同

| 输入语义 | 适配与消费者 | 禁止行为 |
| --- | --- | --- |
| 世界范围与坐标基准 | `world_bounds` 表示该 world 的坐标基准；viewport bounds 单独保存。坐标单位/轴和 world-to-canvas 复用现有专业投影，不能用当前页包围盒重定义世界。 | 分页、搜索或缩放导致整个世界改变尺度/轴向。 |
| `world_binding/source_cut/projection_revision` | 同一 cut 的页组装为一个只读视图；渲染输入附带范围、版本和完整性侧带状态。签名来源先按 API 原合同验签完整投影，再裁剪显示。 | 裁掉未知字段后验签，或混合多个 cut 得到伪完整快照。 |
| 身份/权限 | cache key 包含部署、world、当前主体、audience revision、投影版本、query generation；应用前重验。 | 全局 entity ID 缓存跨世界/账号复用。 |
| 位置来源与时效 | 保留独立 provenance：几何来自权威/地点派生/关系布局/缺失；时效为当前/最近已知/未知/冲突/失效。 | 把 last-known 坐标转成当前 snapshot；刚刷新 relationship layout 被标精确。 |
| 旧 `position_source` | 仅新鲜且权威确认的坐标映射 `snapshot`；明确地点派生映射 `location_derived`；缺失映射 `missing`。其他组合在侧带模型保留来源/时效，并使用明确陈旧/抽象样式。 | 旧 renderer 无法表达时效时静默绘制成当前；此时抑制该动态图形并给文本。 |
| partial/分页 | 按 query scope 合并，保留 coverage/completeness；当前页未出现不是删除。 | 把一页结果送入全量替换路径，清空其他仍有效对象。 |
| 权威删除与本地淘汰 | 同 cut 完整 scope 替换或显式授权 tombstone 才能说明该范围内删除；viewport/LRU 淘汰只是本地不绘制。 | 将视点裁剪、过滤或配额丢弃解释成对象不存在。 |
| 关系/事件 | 独立 authority、端点身份、来源/时效有效才渲染；无空间 anchor 的事件留在 Feed。 | 仅 location_id/邻近生成关系，或给事件找最近 Agent 当位置。 |
| 目标/回执 | TaskContextPanel 来自独立协作流，按绑定显示待同步或原操作结果。 | 用世界帧刷新或当前选中对象改写 task/receipt。 |

`WorldSceneAdapter` 只组装显示输入和状态，不合成完整 Runtime model。现有 Rust 派生函数若必须消费完整集合，应在适配层提供明确的“当前授权可见集”并保持删除/保留语义；不能把 partial 标识丢弃。新增读模型字段属于目标查询合同，不修改运行时世界状态、既有 WASM ABI 或公开旧 snapshot schema。

坐标必须有限且符合既有单位/范围；原始精确整数/签名字段不经过 JS Number 丢精度。需要渲染转换时仅在验签和范围校验后转成局部显示数值，不反写权威坐标。相机、DPR、CSS hit region 与 renderer 投影使用同一转换来源，并用角点、中心、缩放和高 DPR fixture 验证。

## 5. 关键运行流程

<a id="view-query"></a>
### 5.1 进入、探索、选择和跟随

进入先取得部署/world 身份及授权范围，再查询 summary/初始视点。summary 和聚合先过滤授权，未知区域不泄漏隐藏对象精确数量、位置或成员关系。

平移/缩放立即改变本地 camera；稳定后合并只读查询。取消旧请求只是节流，正确性仍由 generation 校验保证。当前请求失败时保留仍被允许的旧范围并标最近已知，不返回空世界。搜索区分当前 Targets 过滤与授权空间搜索；搜索返回稳定 ID、来源与定位线索，不自动控制或提交。

选择/跟随绑定 world/entity 与当前选择代际。显式跟随仅消费当前可信位置；手动浏览退出跟随，位置失效暂停，恢复数据不自动重新夺取 camera。聚合节点下钻、重叠候选列表、返回总览/主目标都只改变读状态。目标无位置时返回文本上下文，不用任意 fallback 坐标定位。

### 5.2 分页、增量和并发

- 首份 snapshot 与 resume cursor 来自同一 source cut。分页 token 绑定 world/audience/query/projection/cut，过期或不匹配则 reset；不能用下一 cut 补旧页。
- 可以渐进展示合法页，但始终标 partial；只有服务器明确完整且全部页验证通过才标 complete。单页200项和2 MiB按实际编码字节约束，超大单项明确 unavailable/resource_limited，不截断字段伪造完整。
- 增量按服务端分区顺序应用，以 world/epoch/stream/event 身份去重。同身份异内容、缺口、失效 cursor、reorg 或不兼容 projection 进入 reset；不按本地时间排序。
- world 与 task 是两个流，分别维护 cursor/binding/revision；交叉 overlay 不一致显示待同步，不拼成原子事务。
- `request_generation` 只表示客户端请求代际，不证明世界版本新旧；current 必须由可验证 source cut/binding 支撑，无法比较的切点不能凭较晚 HTTP 响应覆盖。
- audience 缩小/撤销/换账号时先清受限缓存、图形、详情、搜索建议和在途结果，再初始化新范围；不可先绘制后鉴权。

<a id="view-guidance"></a>
### 5.3 从对象到高层指导

只对当前合法 owner 暴露目标/Prompt/审批/stop 入口。地图、terrain、设施、路线、其他 Agent 的点击不构造 `action:submit`。客户端提示不替代服务端每次授权。

目标保存/activation pending/active/executor read/adoption/context-used 与 world result 分开。审批是明确操作与精确方案摘要，普通 Chat 不是批准。提交沿原 owner API 的 expected revision 与 client_operation_id，世界完成只由匹配原操作/Agent/task 的受权 receipt 确认。窗口关闭不停止独立执行器；模型或 executor 状态不从 renderer health 推导。

<a id="view-admission"></a>
### 5.4 从观察到生效的权威重验

Trace refs: O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3；承接[AC-AGENT-STAGE-002](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-002)。

| 观察/操作 | Viewer 可做什么 | 实际权威边界 |
| --- | --- | --- |
| 当前授权只读 | 展示、定位、检查。 | 服务端读取授权及 source cut；不授予写权。 |
| 最近已知/冲突/未知 | 获准内容可只读；无副作用的建议/草稿明确待核验。 | 不从 View DTO 铸造 Context、grant 或成功；不确定预览不生效。 |
| 激活目标、批准方案或其他高后果操作 | 以固定主体、原草稿/方案和 expected revision 提交；旧观察说明可见。 | [Game API Context](../llm/external-agent-game-api.design.md#api-context)与[操作 admission](../llm/external-agent-game-api.design.md#api-actions)重验当前主体、授权、控制/目标版本、Context/fence和适用世界切点；资源/规则由 Runtime 最终裁决。 |
| 校验前后来源/权限变化 | 明确 stale/conflict/reauthorization；需要改变方案时重新预览/确认。 | 校验必须与接受/生效的权威事务关联，不能是 Viewer 单独 GET 后即视为安全。 |
| stop/撤销/原结果查询 | 保持独立的安全控制/恢复入口，不因图形失败统一置灰。 | 各自 owner 授权及操作合同；观察陈旧不自动撤销已有合法控制，撤权也不能被绕过。 |
| 响应丢失/结果未知 | 保留原操作身份，显示 recovery_required 并回查。 | 不生成新键、不换 Agent、不重报价替换已缓存请求；按原 API 幂等恢复。 |

接线过程为：固定主体与方案 → 读取现有权威上下文/可用能力 → 纯预览（如适用） → 明确确认 → 原 API 的版本约束提交 → pending/拒绝/结果待确认 → 受权回读原操作。Viewer 只携带既有 API 要求的字段；`observed_at_tick`、UI freshness badge 或 WorldView cursor 不是 admission 许可。真正执行世界动作的 Agent/执行器仍按自己的 Context/fence 接受 Runtime 校验，owner guidance 不绕过该层。

必须记录可脱敏关联的原操作、显示使用的 cut/来源分类、expected/实际控制版本和权威拒绝原因，供测试对账；不要求公开内部 token、观察正文或私有推理。

## 6. 接口与数据合同

路由目标仍为 Game API `/v1/game/worlds/{W}/views`，共享现有鉴权。以下是目标字段，不声明端点已部署；精确 wire schema 在实现中与 API owner 一起锁定，并严格处理不支持版本，不能让旧 consumer 猜默认成功。

```text
WorldViewQueryV1 = {api_revision, request_generation, scope_ref,
  scale: world|region|local, bounds?, focus_entity_id?, filters,
  page_size, cursor?}
WorldViewSnapshotV1 = {world_binding, snapshot_id, projection_revision,
  audience_revision, source_cut, request_generation, coverage,
  entities, relations, events, next_cursor?, resume_cursor,
  completeness: complete|partial|unknown, unavailable_reasons}
EntityViewV1 = {entity_id, kind, parent_ref?, label,
  position?, position_source: authoritative|relationship_layout|last_known|unknown,
  observed_at_tick?, valid_at_source_cut, visibility, activity?,
  task_refs, receipt_refs, authorized_guidance_capabilities,
  provenance?}
provenance = {geometry_source: authoritative|location_derived|relationship_layout|missing,
  freshness: current|last_known|unknown|conflict|expired,
  source_ref, observed_at_tick?, valid_at_source_cut, reason?}
```

`provenance` 将几何与时效分开，旧组合 `position_source` 仅作兼容表达。缺 provenance 的旧输入最多保留其可证含义，不能单凭 `authoritative` 升级为当前动态观察；若有效的 world binding/观察 authority 不能证明 current，则降级 unknown/last-known并阻止相关生效。新增必填语义需要 API 版本协商/显式能力；已发布 consumer 不支持时 blocked/升级，不以 optional 默认值绕过。

实体类型、物理单位、合法关系复用 Runtime 与[语义定位](viewer-pixel-world-semantic-positioning.prd.md)。tick/revision 采用 API 的精确十进制表示；activity、Intent、receipt 不混用；unknown 不填 `(0,0)`，不可见对象不返回私有 label/计数。

`POST /query` 只读；`GET /entities/{id}` 重新鉴权并绑定 cut；`GET /search` 对 q/filter/cursor/返回条数有界；`GET /events` 只投影获准事件。offset 不能替代同切点 cursor。缺空间 anchor 的事件只进 Feed。

错误语义沿 API 错误 envelope：身份/权限问题不披露对象存在性；cursor 失效需要重新 snapshot；cut/revision 冲突需要重新确认；资源限制返回可解释的 partial 或明确拒绝；服务不可用不伪装空集合。客户端区分可重试的只读请求与必须先回查的副作用，退避带抖动并有次数/总 deadline，不无限自动刷新。

## 7. 状态、事务与持久化

camera/selection/filters/drawer 是展示偏好，可按当前账号/world保存轻量非秘密设置；任务、审批、原操作、结果不以 localStorage 为共享真值。撤权/换账号清除旧私有缓存和恢复键；不能恢复旧 selection 后泄漏详情。

服务端空间索引是可重建投影，索引 generation 与响应 source cut 对齐。缺 checkpoint/root、索引落后或无法验证绑定时不给 current。服务端先授权再聚合/计数；读投影重建不修改世界历史。

UI 分别保存 network、renderer、query/coverage、task/goal、executor、world operation 状态。renderer failure 不变成 executor offline，已 committed 结果不因重载改成普通 pending。缓存有按字节/对象数的 LRU 上限，pin只影响显示优先级，不绕过授权或无限扩容。

<a id="view-recovery"></a>
### 7.1 降级与恢复状态机

| 状态/触发 | 清理/保留 | 恢复条件 |
| --- | --- | --- |
| booting/loading | 保留稳定舞台框、合法目标和文本。 | 正确 hash 的资源、当前读模型与实际 renderer 就绪。 |
| 初始 GPU 不可用/资源缺失 | 明确 unavailable；无第二 JS 世界。 | 显式重试原可信资源或保持文本/Targets 降级。 |
| ready 后 context lost/fatal | 停止旧图形命中和动画；旧帧不标实时，保留获准文本/原操作。 | 重建图形会话，重新应用当前授权视图和 camera，再验证命中；不能只看 probe。 |
| disconnected | 数据标 stale，受限操作按 authority 判断；保留可披露历史。 | cursor/binding/audience有效才增量恢复，否则重取 snapshot。 |
| query failed/partial | 只标受影响范围，旧可披露范围不被误删。 | 重新查询该 scope，完成前不标 complete。 |
| reorg/gap | 丢失效游标/索引，结果显示按 Runtime 重组状态。 | 新 checkpoint及配套 cursor；不重放成新收益。 |
| audience/账号/world变化 | 先清私有缓存、画布、详情、搜索/hover与旧 promise结果。 | 新授权完成后初始化，不沿用旧权限。 |
| 原结果未知 | 保留原 operation identity 和 recovery_required。 | 查证原结果；无证据不创建第二次效果。 |

图形自动恢复最多一次有界尝试，失败后给显式重试/文本路径，不启动无限 reload。页面/窗口关闭只清本视图订阅、GPU和临时缓存；不取消独立 worker，不杀其他进程。

## 8. 部署、安全与运行约束

<a id="view-native"></a>
### 8.1 NativeWorldViewHost 与目标平台矩阵

在现有 Launcher 提供受限世界窗口，加载与 Web 同一有内容 hash 的 Viewer build，而非弹系统浏览器。目标实现选择系统 WebView：macOS 对接 WKWebView，Windows 对接 WebView2；Linux 内嵌方案单独作为候选，未验证不宣称支持。本表是新增世界窗口的实施顺序，不改写其他已发行组件的支持范围。

| 组合 | 设计选择 | 上线前必须锁定 |
| --- | --- | --- |
| Web 桌面 | canonical Viewer，共享 Game API。 | 浏览器/OS/GPU/版本、viewport/DPR、release build与实际交互。 |
| macOS native | 受限 WKWebView 世界窗口。 | Rust binding及锁文件、OS/架构、WebView/WASM/WebGL启动、签名发行资产。 |
| Windows native | 受限 WebView2 世界窗口。 | binding/runtime分发模式、OS/架构、GPU恢复、安装/升级/回滚资产。 |
| 其他浏览器/移动/Linux | 只按单独通过的组合扩大。 | 全部同义旅程与安全/质量证据，不能继承上述 green。 |

依赖和最低版本由 Launcher owner 在实现候选锁定；这是可执行的依赖交付项，不用“纯数据 Launcher 已存在”当内嵌浏览器完成。未通过组合保持 unsupported/未验证，不静默打开外部浏览器计为通过。

### 8.2 来源、认证与 IPC

远程世界使用受信 HTTPS 同源页面；本地世界使用显式启动的 loopback 服务。导航和重定向按部署 allowlist 校验，拒绝任意第三方页面获取 native bridge，不禁用 TLS 校验。IPC仅窗口生命周期、用户明确保存文件与安全存储，不提供 shell、任意文件读或 world action后门。

认证复用现有 device/session proof；一次性 bootstrap 绑定 trusted origin、nonce、窗口，不把 secret放URL。远程 browser cookie按既有 Secure/HttpOnly/SameSite/CSRF策略；native不共享模型token缓存。深链只含 world/entity/task，重新鉴权后只读定位，不携带 bearer、不自动批准/提交。

### 8.3 资产、发布与可观测性

遵守 optional WASM/静态资产 manifest、SHA-256和build provenance；页面/模块/字体/可选payload版本不一致时明确失败，不下载任意代码修复。可选 payload缺失只影响图形，不把合法task/receipt/control清空。发布记录区分核心UI可用、renderer ready和完整探索，不把安装包存在当可玩。

指标记录读取/解码/派生/reconcile/首帧/输入、payload字节、cache、reset原因及renderer失败阶段，标签有界。诊断用脱敏关联ID和理由，不记录原token、prompt、私有label/坐标或其他账号数据；无埋点不声称达标。

## 9. 质量与容量

<a id="view-quality"></a>
### 9.1 容量合同

单页最大200项、完整编码响应最大2 MiB；单视点累计最多1000个实体投影，最多突出200个labels。目标/选中/关键blocker/相关路线优先；超过预算使用授权聚合/分页和partial，不能不加说明地吞掉关键对象。单个目标详情可另查，但不无限增加画布对象预算。包含关系端点和聚合成员的真实负载应分别记录。

服务端使用可重建空间索引/有界scope查询，先授权后聚合。客户端合并/取消读请求、缓存有界、将静态reconcile与动画分开；renderer不因每个包重建全部实体。全世界50000对象是测试世界规模，不是每帧传输/绘制50000对象的承诺。超限/洪泛不阻塞输入、退出跟随、stop和原结果查询。

### 9.2 初始验收预算

下表是待测的实施目标，不是现有性能报告。基准 A 为桌面8 GiB内存、集成GPU、100 Mbps/40 ms RTT；基准 B 为同一设备受限10 Mbps/150 ms RTT。实际CPU/GPU/OS/浏览器、电源与显示刷新率必须记录；缺硬件身份不能比较。DPR/视口和负载见专项手册。

| 指标 | 初始目标 | 测量边界 |
| --- | --- | --- |
| 核心目标/反馈/恢复界面可操作 | A冷启动P95≤3 s；B≤5 s。 | 从已完成登录且发起页面导航至能操作已授权文本上下文；登录时间另报。不得等待可选renderer。 |
| 首个真实可交互世界帧 | A冷P95≤20 s、热≤5 s；B冷≤90 s、热≤10 s。 | 包含实际payload下载、解码/编译、snapshot、派生与首帧；不把占位图/DOM算入。当前可选WASM较大，冷/热必须分别报实际传输字节。 |
| 有界query服务耗时 | P95≤300 ms；端到端另记录网络与响应体。 | 50000对象测试世界、获准scope；不以缓存命中替代冷索引测试。 |
| 本地选择/退出跟随/stop控件反馈 | P95≤100 ms；不出现连续1 s无法响应输入。 | 只表示界面接受操作，不等于网络提交/Runtime完成。 |
| 持续平移/缩放可见帧间隔 | 桌面P95≤33.3 ms，P99≤100 ms。 | 1000实体/200labels的前台交互窗口；静止按需不渲染不算掉帧，后台节流单列。 |
| 数据/空间缓存 | 应用可归因缓存≤64 MiB且有对象上限；累计renderer进程内存另测。 | 超限按scope/LRU降级，不删权威结果或泄漏旧账号数据。 |
| 30分钟往返/选择长稳 | 无崩溃、无旧命中残留；同负载稳定后末窗口内存较初窗口增长≤10%。 | 记录GC/渲染资源测量方法与不可测项，不能只报JS heap代替WASM/GPU。 |

首次实测不达标时报告瓶颈并修复/缩小该组合的支持范围；必要预算调整必须有相同fixture的证据和owner决策，不在测试时临时放宽阈值。以上预算不是新通用CI gate；普通文档PR不运行GPU或付费模型。

### 9.3 可读性与密度策略

以选中/当前目的/blocker/相关路线优先，背景terrain按屏幕LOD，非关键labels渐退；确定性散布不能单独证明重叠可用，必须提供候选选择入口。关键状态至少有文本/形状等非颜色信号。无精确来源的坐标不用于距离/路线建议。首读5秒辨认与误归因负例按专项手册，模型视觉审读不代替真人理解/L5。

## 10. 兼容、迁移与回滚

1. 先修现有HUD主体绑定与时效表达，补Rust/host负例；不改变已有action权限。
2. 接授权一致snapshot和task overlay，完成分页/partial/代际适配；保留旧有界snapshot路径时明确其适用范围，不同时拥有两个真值。
3. 扩空间查询、聚合、选择/跟随与性能，旧DTO不能表达语义时文本降级而非猜默认。
4. 同一Viewer build接原生容器，逐OS/浏览器/资产和恢复验收后扩大支持声明。

回滚只切兼容Viewer build或文本降级，不改世界历史、不清pending、不恢复旧授权、不静默切debug世界/旧executor。projection major不支持时blocked/升级；恢复旧构建前检查其能否正确表达来源、权限与原操作，否则仅文本模式。P1-A/B/C事实前置、World Feed与receipt分离持续有效。

## 11. 验证设计与可追溯性

普通CI覆盖类型/精度、来源映射、部分页/删除、乱序、撤权、selection、原主体和地图无副作用；fake context/操作账本测试不调用模型。实际S6/GPU/原生/真人理解另取证。沿用[外部Runtime组合手册](../../testing/manual/external-agent-runtime-contract-validation.manual.md)的viewer-*场景，并用[像素世界专项手册](../../testing/manual/pixel-world-visualization-validation.manual.md)补足可重复判据，不能缩减原AC。

### 11.1 验证映射表

| 上游 requirement / AC | 本设计条款 | 独立 obligation 与条件 | 验证方法 / scenario / source / environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-001) | [view-scene](#view-scene) | 首读/归属与环境分离。 | [PW-IDENTITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-identity)、[PW-READABILITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-readability)；Rust/host及实际S6同候选。 | 输入/原操作/DTO/截图/回答，分别记录层级。 | DOM或模型审读不证明真人理解。 |
| [AC-AGENT-STAGE-002](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-002) | [view-admission](#view-admission) | 冲突/过期/撤权重验，未知原结果不重做。 | [PW-FRESHNESS](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-freshness)；确定性竞争+实际API/Runtime临时世界。 | 显示cut、原operation、权威重验/拒绝、前后结果。 | fake authority不证明真实接线。 |
| [AC-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-003) | [view-adapter](#view-adapter)、[view-query](#view-query)、[view-quality](#view-quality) | 同cut/partial/坐标与多尺度可用。 | [PW-PROJECTION](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-projection)、[PW-EXPLORATION](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-exploration)、[PW-PERFORMANCE](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-performance)。 | 页集合、删除/淘汰差异、实际帧/查询/内存与交互。 | 阈值单测不证明容量或像素。 |
| [AC-AGENT-STAGE-004](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-004) | [view-native](#view-native)、[view-recovery](#view-recovery) | 各端界面内交互及失效恢复。 | [PW-PLATFORM](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-platform)、[PW-RECOVERY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-recovery)；实际release/WebView/OS。 | 启动资产hash、图形前后、交互/原操作、平台。 | 另一端/GPU-disabled不代签ready路径。 |
| [AC-AGENT-STAGE-005](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-005) | [view-guidance](#view-guidance) | 观察无副作用；合法指导与原结果。 | [PW-IDENTITY](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-identity)、[PW-FRESHNESS](../../testing/manual/pixel-world-visualization-validation.manual.md#pw-freshness)；双主体/原操作临时世界。 | 前后world root、请求类别、原receipt和权限。 | UI disabled不证明服务器授权。 |
| [AC-EXT-015](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-015) | [view-native](#view-native) | 两surface分别使用同世界。 | [viewer-dual](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-dual)及PW-PLATFORM；真实候选。 | 双端操作/世界结果。 | 不单独证明真实模型或完整首局。 |
| [AC-EXT-016](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-016) | [view-query](#view-query) | 从概览至局部再回到目标。 | [viewer-explore](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-explore)及PW-EXPLORATION。 | 授权对象/来源/位置与返回路径。 | 不证明全图可见。 |
| [AC-EXT-017](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-017) | [view-guidance](#view-guidance) | 世界现场合法指导/回执。 | [viewer-guidance](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-guidance)及PW-IDENTITY。 | owner操作与原receipt关联。 | 不以accepted/animation代签世界效果。 |

## 12. 决策、长期风险与未决问题

ADR-VIEW-1：共享Viewer/读模型，native采用受限系统WebView，代价是逐平台资产与图形验证。

ADR-VIEW-2：只读空间浏览与owner高层指导分开，服务端裁决，代价是缺来源/权限时必须诚实降级。

ADR-VIEW-3：几何来源、新鲜度、请求代际和世界切点分别保存；partial不是全量，显示默认值不成为业务主体。代价是适配/缓存比直接映射snapshot更严格，但避免旧位置、旧权限和错归属。

ADR-VIEW-4：先保持现有renderer和Shell，预算驱动局部索引/聚合/reconcile优化，不先重写引擎或shader。

实施仍需锁定各平台依赖/发行身份、现有API Context/fence的实际接线、空间索引数据来源及真实质量基线；这些是明确的交付缺口，不是可跳过的验收。独立跨域评审与运行证据在实现PR保留，本文不自报其已经完成。
