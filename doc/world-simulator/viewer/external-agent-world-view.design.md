# 外部 Agent 大世界 Web 与桌面可视化系统设计

- 设计状态：target contract，待实现与独立评审；不是当前能力或发布报告。
- Canonical repository：`eng-cc/oasis7`；审读基线：`03fb58784a65de9f02a43dfeafc3e882bb04e5f2`（已合入产品 PR #4493）。
- Owner role：`viewer_engineer`；最后审读：2026-10-10；新增合同 revision：`1`。
- 产品入口：[外部 Runtime 游玩](../../product/agents-world-simulation/external-agent-runtime-play.prd.md)；[玩家与 Runtime 协作](../../product/agents-world-simulation/player-runtime-collaboration.prd.md)。
- 系统总入口：[LLM / external runtime 系统入口](../llm/README.md)；写作规范：[系统设计规范](../../engineering/doc-governance/system-design-writing-standard.design.md)。

本文中的新组件、路由、记录和测试场景均为后续实施输入。`accepted` 只表示所属权威已受理，世界效果必须读取既有权威回执；文档合入不代表这些接口已经上线。

## 1. 问题、目标与非目标

玩家需要在网页和桌面客户端内探索正在被外部 Runtime 驱动的同一持久世界，而非只看 Agent 日志、旧世界截图或一个打开浏览器按钮。本设计定义共享读模型、空间查询、选择/跟随、任务叠加、native 容器和故障降级。

非目标：另建世界引擎、通过地图直接采集/移动/建造、所有玩家全图实时可见、恢复退役的第二 Viewer 工具链或一次实现所有地形/事件数据。空间规则与阶段依赖消费现有世界舞台 PRD，缺失来源必须诚实显示未知。

## 2. 上游约束与相关角色

Viewer/视觉 owner 维护共享场景和交互；Runtime owner 提供 authoritative world binding、位置/关系及回执；身份 owner 提供 audience 和撤销语义；Launcher owner 负责实际客户端容器与发行平台。浏览器与 native 各自取证，不互相代签。

### 2.1 需求承接与分配表

| 上游条款 | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 排除与未覆盖范围 |
| --- | --- | --- | --- | --- |
| [AC-EXT-015](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-015) | Web 与桌面各自实际操作同一真实世界 | [view-native](#view-native) | viewer_engineer / qa_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-016](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-016) | 从总览浏览局部并返回目标，未知信息不伪造 | [view-query](#view-query) | viewer_engineer / runtime_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-017](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-017) | 世界现场只连接合法高层指导与权威回执 | [view-guidance](#view-guidance) | viewer_engineer / agent_engineer | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-001) | 目标、因果与下一步优先于装饰和日志 | [view-scene](#view-scene) | viewer_engineer / game_visual_interaction_designer / qa_engineer | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-002](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-002) | 陈旧、冲突与范围外观察保持来源语义 | [view-query](#view-query) | viewer_engineer / game_visual_interaction_designer / qa_engineer | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-003) | 多尺度定位与对象检查保持任务上下文 | [view-query](#view-query) | viewer_engineer / game_visual_interaction_designer / qa_engineer | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-004](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-004) | 网页和原生客户端都具备界面内交互 | [view-native](#view-native) | viewer_engineer / game_visual_interaction_designer / qa_engineer | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-005](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-005) | 选中可见对象不授予新的控制权 | [view-guidance](#view-guidance) | viewer_engineer / game_visual_interaction_designer / qa_engineer | 不证明所有平台、全图可见或公开发行 |

## 3. 当前状态、目标状态与差距

| 基线 | 已知事实 | 新设计目标 / 不可外推 |
| --- | --- | --- |
| Viewer | `viewer-manual.manual.md` 明确 canonical `crates/oasis7_viewer/viewer.html` 与 `oasis7_viewer_live` | 扩展同一 Viewer，不建立第二份 world state |
| 世界读回 | 手册说明 runtime writer 与 Observer 使用可验证 immutable generation checkpoint | 新聚合/空间读模型必须绑定这一来源，不能从进程内旧场景合成当前世界 |
| 当前显示 | 现有舞台产品文档明确旧 2D overview/zoom 已退役，P1-A/B/C 有事实前置 | 新多尺度设计是 target，不能宣称现行已可用 |
| Native | launcher 控制面及 `oasis7_launcher_ui` 并不证明客户端内大世界渲染 | 增加隔离 WebView host 和实际内嵌 Viewer；运行证据待补 |

## 4. 边界与结构

<a id="view-scene"></a>
一个共享 Viewer 应用负责 world-first 场景、任务详情、消息/审批和恢复；Web 与 native 加载同一有内容 hash 的 Viewer build。共用类型与应用状态，不让两入口各自计算资源余额或判定任务完成。

分为四层：`WorldProjectionReader`（服务端按授权生成读模型）；`WorldDataClient`（分页、快照/游标、重连与请求代际）；`WorldSceneAdapter`（既有 renderer 的空间、LOD、选择与回退）；`TaskContextPanel`（来自协作 snapshot 的目标/问答/审批与回执引用）。任务控制 UI 不取代世界主舞台；其来源和时效独立于画面刷新。

数据流是 Runtime checkpoint/events → 授权读模型 → Web/native；玩家指令是受控 owner API → Runtime control commit，不能写 renderer 内存后显示世界成功。外部 executor 的私有 observation 不能直接广播给 Viewer；公开/私有 audience 分开投影。

## 5. 关键运行流程

<a id="view-query"></a>
### 5.1 从世界总览到局部

进入页面先取得部署身份和许可范围，再请求 world summary/初始 viewport query。summary 只聚合可披露对象，未知部分明确 unknown，不披露隐藏对象精确数量或位置。视点的平移/缩放只修改本地 camera；稳定后以有界区域/尺度发只读查询，旧 request_generation 返回时丢弃，不覆盖最新视点。

世界/区域/Fragment/对象的层级取自权威关系或明确标识的派生布局；坐标缺失时可以用关系位置展示，但标记 position_source=relationship_layout，不能用于计算可执行路线/距离。Last-known 与当前 confirmed 的样式和可访问文本不同；未知不是零资源或空世界。

搜索返回许可范围内稳定 entity ID 与定位线索；select/follow 绑定 world_id、entity_id 与当前 query generation。跟随丢失可信位置时停止自动追踪并显示 last-known，可退出跟随或回到主目标，不默认选择另一个 Agent。

### 5.2 增量与并发响应

初始 snapshot 与 event cursor 必须同一 source cut；增量按 stream 分区顺序应用。world 流和 task 协作流是不同 cursor，不用客户端时间把它们拼成原子世界事务；跨流 overlay 保留各自 WorldBinding/goal revision 与“待同步”状态。

分支/reorg/generation 不匹配、cursor 失效、权限 revision 改变或事件缺口时，清除受影响缓存并重取 snapshot。不能先把旧 owner 私有内容绘制后再等待鉴权；数据层在应用响应前重验当前 audience/selection generation。断连只使后续来源变 stale，不撤销已确认历史成果。

<a id="view-guidance"></a>
### 5.3 从对象到高层指导

点击 Agent 打开经过服务器授权的 task context；有当前 owner 权限才提供目标/Prompt/审批/stop 入口。点击地图、地形、工厂、路线或选择其他 Agent 只作观察，不构造 action:submit。客户端权限提示仅是体验，服务器每次写入仍重验。

提交目标携带 expected revision 与原 client_operation_id；显示 saved、activation pending、active、executor read/adoption/context-used 的真实区别。审批必须是明确操作与精确方案摘要，普通 Chat 输入不能批准。世界完成显示必须通过受权 receipt 回读并匹配 Agent/任务，不能引用别人的生产结果或用 animation 代替。

## 6. 接口与数据合同

路由统一挂在 Game API `/v1/game/worlds/{W}/views`，不新增第二个 auth 系统。

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
  task_refs, receipt_refs, authorized_guidance_capabilities}
```

实体类型/物理单位/合法关系复用 semantic positioning、Fragment LOD 和 Runtime schema；JSON 的 tick/revision 与 API 保持十进制字符串。`activity` 与 receipt 是不同字段，unknown 不使用默认坐标补齐。授权过期的 reply 不返回差异及原私有 label。

`POST /query` 只读；`GET /entities/{id}` 按主体重新鉴权；`GET /search` 有界 q/filter/cursor；`GET /events` 只投影获准世界事件。一个 scope/page 不足以覆盖所有信息时给 next_cursor 与 completeness，不截断后返回 complete。未具备空间 anchor 的事件显示在相邻世界 feed，不伪造 map marker。

## 7. 状态、事务与持久化

camera、selection、filters、drawer 是客户端展示状态；可以按账号/world 保存轻量偏好，但不是世界事实。切换账号或 audience 缩小时删除旧私有缓存，不能跨账号恢复 selection 后泄漏详情。

服务端聚合和空间索引是可重建读投影，checkpoint/root 不完整时拒绝提供 current；后台索引 generation 必须和响应 source cut 对齐。任务目标、审批、operation、回执由对应权威保存，不写 localStorage 作为共享真值。

UI 相互独立保存：网络状态、renderer 状态、当前 task/目标、executor 通信状态和 world operation 状态。renderer failure 不能变成 executor offline；已经 committed 的结果不会因页面重载显示 pending 或触发重做。

## 8. 部署、安全与运行约束

<a id="view-native"></a>
### 8.1 NativeWorldViewHost

拟新增原生 host 作为现有桌面 Launcher 的受限世界窗口，使用系统 WebView 容器加载同一官方 Viewer build，而不是启动系统浏览器。首期架构选择共享 Web 渲染实现；WebView binding 的具体依赖版本由 Launcher owner 在实施锁定、平台试验后提交依赖，不能把当前纯数据 `oasis7_launcher_ui` 当已包含浏览器内核。

远程世界加载受信 HTTPS 同源页面；本地世界加载显式启动的 loopback 静态/API 服务。所有导航按部署 allowlist 校验，拒绝任意第三方页获取 native bridge；生产不允许禁用 TLS 校验。native IPC 只承载窗口生命周期、用户明确的文件保存和安全存储操作，不提供通用 shell、任意文件读取或 world action 后门。

登录继续现有 device/session proof；一次性 bootstrap 必须绑定 trusted origin、nonce 和该窗口，secret 不放 URL。远程 browser cookie 使用 Secure/HttpOnly/SameSite 与 CSRF 校验；native 缓存不得共享模型 token。公开深链只含 world/entity/task 标识，不能携带 bearer 或隐式授予 owner 权限。

### 8.2 渲染资源与失效

沿当前 Viewer 的可选 WASM/静态资产 provenance 规则，包 hash 和版本需要与实际页面一致；缺 optional payload 则 RendererUnavailable 并保留任务文本、可信回执和返回入口，不下载任意动态代码补救。图形崩溃只重建渲染会话，不重置授权和任务。

Web 与每个声明支持的桌面 OS 均有实际资产启动和操作证据。没有通过的平台不得在支持矩阵中自动继承 Web 的 green；客户端弹浏览器不能算 native 通过。

## 9. 质量与容量

大世界按视点有界查询，API page 最大 200、总响应 2 MiB。首期 Viewer 单视点最多累计 1000 个实体投影，超过改用聚合/分页并标 partial；最多突出 200 个 labels，当前目标/选中/关键 blocker 优先，不能用隐藏信息创造“对象不存在”的结论。

先用同一候选下的 50000 对象、多区域测试世界测绘制帧耗时、P95 查询、缓存内存和切换延迟；这是验收负载设计，不是现有并发能力声明。请求合并/取消只影响只读 query，不取消世界 operation；大列表和事件洪泛不阻塞输入、退出跟随或 stop/授权操作。

## 10. 兼容、迁移与回滚

复用 canonical Viewer，不恢复退役第二 Viewer。先接受一致 snapshot/task overlay，再扩多尺度/选择，随后 native 内嵌与独立验收。保留现有 P1-A/B/C 的身份/来源/事件前置，尚未具备 anchor 的对象以未知或关系模式显示，不修改游戏规则补齐 UI。

回滚只切回可兼容的 Viewer build 或文本降级，不能改变世界 schema、丢 pending、恢复旧授权或静默打开调试世界。旧 projection major 不受支持时明确升级/blocked，禁止用旧字段填充新响应假装完整。

## 11. 验证设计与可追溯性

普通 CI 覆盖类型、缓存代际、乱序响应、权限收窄、selection、地图只读和 controlled command。S6/实际桌面资产验证真实浏览器、WebView、可访问文本、交互与渲染故障；截图必须配相应状态/操作结果，不单独证明世界成功。

### 11.1 验证映射表

| 上游条款 | 本设计条款 | 独立 obligation 与适用条件 | 准确验证方法与场景 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-EXT-015](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-015) | [view-native](#view-native) | Web 与桌面各自实际操作同一真实世界 | [viewer-dual](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-dual)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-016](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-016) | [view-query](#view-query) | 从总览浏览局部并返回目标，未知信息不伪造 | [viewer-explore](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-explore)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-EXT-017](../../product/agents-world-simulation/external-agent-runtime-play.prd.md#ac-ext-017) | [view-guidance](#view-guidance) | 世界现场只连接合法高层指导与权威回执 | [viewer-guidance](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-guidance)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不单独证明真实模型、客户端或公开发行就绪 |
| [AC-AGENT-STAGE-001](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-001) | [view-scene](#view-scene) | 目标、因果与下一步优先于装饰和日志 | [viewer-primary](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-primary)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-002](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-002) | [view-query](#view-query) | 陈旧、冲突与范围外观察保持来源语义 | [viewer-freshness](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-freshness)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-003](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-003) | [view-query](#view-query) | 多尺度定位与对象检查保持任务上下文 | [viewer-scale](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-scale)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-004](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-004) | [view-native](#view-native) | 网页和原生客户端都具备界面内交互 | [viewer-native](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-native)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明所有平台、全图可见或公开发行 |
| [AC-AGENT-STAGE-005](../../product/agents-world-simulation/player-readable-world-stage.prd.md#ac-agent-stage-005) | [view-guidance](#view-guidance) | 选中可见对象不授予新的控制权 | [viewer-authority](../../testing/manual/external-agent-runtime-contract-validation.manual.md#viewer-authority)；按该场景的候选、环境、故障注入和判据执行 | 对应 source HEAD 的 CI 输出或脱敏实测记录，保留世界回执关联 | 不证明所有平台、全图可见或公开发行 |

## 12. 决策、长期风险与未决问题

ADR-VIEW-1：两使用表面共享 Viewer 和读模型，native 采用内嵌世界窗口；不再开发两套独立 renderer 或世界缓存权威。代价是需要逐 OS 验证 WebView 与图形资产。

ADR-VIEW-2：只读空间浏览与 owner 高层指导分开；存在可见对象不代表其可控制。渲染降级优先保留决策锚点而非伪造像素世界。

实施前 Viewer/Launcher owner 必须锁定可支持的 OS/WebView 依赖和实际世界 projection 数据来源；Runtime owner 提供可复核的 snapshot/event cut。数据源或平台缺口只允许降低该组合的支持声明，不能通过 iframe、外部浏览器或模拟截图替代要求。
