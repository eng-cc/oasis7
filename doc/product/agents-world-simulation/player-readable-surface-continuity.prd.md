# 玩家可读表面的连续性

## 文档身份

- 所属产品模块：智能体、世界模拟与交互
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/player-readable-surface-continuity.design.md`](player-readable-surface-continuity.design.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md)、[`world-simulator PRD`](../../world-simulator/prd.md)、[`Web 语义测试 API`](../../world-simulator/viewer/viewer-web-semantic-test-api.prd.md)
- Last reviewed：2026-09-14

本文定义正式玩家表面在 viewport、信息密度、语言与连接状态变化时保持可理解、可操作和可恢复的长期产品承诺。它不冻结 Viewer 布局、组件、Web/EGUI/Bevy 实现、协议字段、缓存、字体资产或当前发布结论。

## 设计适用性与生命周期闭合

- 设计判定：`paired-design`。
- 配对关系：[player-readable-surface-continuity.design.md](player-readable-surface-continuity.design.md) 承接本 PRD 的玩家经历、信息层级、状态反馈与恢复解释；产品真值仍由本 PRD 拥有。
- 设计适用性理由：跨 surface 的阅读锚点、连接状态、fallback 与行动反馈由同名 design 承接。
## 1. 目标与产品承诺

玩家在受支持的表面中能够持续找到当前目标、主要 blocker、已接受行动的反馈和下一决策。viewport、信息密度、语言或连接状态变化可以改变呈现方式，但不能让玩家失去这条决策链，也不能把恢复中的界面或后台活动表达为已经取得进展。

当前世界模式可支持的动作集合也属于这条决策链。surface 必须让玩家辨认当前可以做什么；动作不受当前模式支持时，应如实说明这是能力或世界边界，并给出适用的替代动作或安全返回路径，不能把它伪装成断线、发送失败或已经执行。

## 2. 可用空间与信息密度

- 在受支持的 viewport 尺寸下，主要世界或行动表面保持可读和可操作；面板、设置和次级信息可以折叠、隐藏或重排，但可支持的决策面必须能够被发现和恢复。
- 玩家可以降低信息密度或收起次级内容，且不会因此永久丢失当前目标、主要 blocker、接受/结果反馈或下一步。
- 遮挡、窄屏或低高度不能让关键状态只能依赖 hover、颜色、动效或不可恢复的布局位置。
- surface 可以选择全屏、分区或响应式组合，但产品不承诺具体 toggle、panel、module、minimap、toast 或历史 Viewer 控件存在。

### 2.1 当前模式与行动边界

- 当前 surface 只呈现或接受当前模式确实支持的行动；不支持的行动与连接中断、权限拒绝和已接受请求是不同的玩家语义。
- 模式、布局或可见动作集合变化不会创造回退、重放、权限或世界修改能力；玩家仍能获得一条真实的替代动作、等待、返回或重新聚焦路径。
- 行动入口、时间展示或自动推进可以变更，但界面计数、后台调度、重新加载或自动开始不能代替权威世界后果。

## 3. 语言与可访问文本

- 受支持的玩家语言具有明确的选择与 fallback；必要的意图、状态、错误、恢复和下一步文本保持可读。
- 语言选择只改变表达，不改变世界事实、动作权限、接受/拒绝语义或执行结果。
- 缺少翻译或字体覆盖时，surface 必须诚实降级并保留关键含义，不能以空白、乱码或静默回退隐藏状态。
- 本产品承诺不包含自动采用操作系统 locale、特定字体资产、云端或跨设备同步，也不冻结翻译键、缓存路径或持久化机制。

## 4. 连接中断与恢复

- 连接中断、重连中、恢复成功或恢复失败必须可区分，并提供适用的重试、返回或重新进入路径。
- reconnect、重新加载、后台 tick 或重新出现的画面不能代签玩家意图已接受、动作已完成或世界已推进。
- 恢复后，玩家能够重新找到当前目标、主要 blocker、最近可信反馈和下一决策；无法恢复时明确返回安全的决策入口。
- websocket、callback、backoff、timeout、software-safe、WASM 或 transport 兼容由专业域拥有，产品层不复制。

### 4.1 重叠状态的主语义与真值优先级

多个状态同时出现时，surface 不能用最近到达的 transport 事件、页面刷新或客户端计数覆盖已经确认的世界语义。默认主语义按下表决定；其他仍有决策价值的状态可以作为次级 blocker 展示，但不得改写主语义。

| 重叠条件 | 主语义与玩家可见边界 | 允许的下一步 | 禁止的推断 |
| --- | --- | --- | --- |
| 已有 committed receipt，同时断连或恢复中 | 以已确认结果为主；连接只影响查看后续状态，不撤销结果 | 查看 receipt、等待重连或进入新的受支持决策 | 把断连表示为回滚、未执行或需要自动重放 |
| 请求已被接受但尚未 committed，同时断连或恢复中 | 以“待决、尚无世界效果”为主；断连是恢复 blocker | 重连后查询、等待，或在专业合同允许时明确撤回/替代/重规划 | 把送达、界面计数或重连成功表示为成功、失败或取消 |
| 请求尚未被权威接受，同时权威不可用、陈旧或无法验证 | 以“未确认/需重新验证”为主，不提前判定接受或拒绝 | 重新验证、等待、改道或安全返回 | 把本地排队、请求发送或缓存响应表示为拒绝、成功或资格 |
| 当前行动在模式中不受支持，同时连接中断 | 以“能力边界”为主；连接状态可独立展示给其他受支持行动 | 选择当前模式支持的替代、等待或安全返回 | 把不支持误报为传输失败，或因重连制造该行动能力 |
| 已有权威拒绝/规则或权限阻塞，同时随后断连 | 以拒绝或阻塞结果为主；断连只影响查看详情或重新评估 | 查看原因、修复前置、改道或在条件允许时显式重新提交 | 自动重试、把拒绝改写为断连失败，或继承旧权限 |

该优先级只规定玩家的默认首要解释，不冻结字段、状态枚举或 UI 排版。任何新的提交仍须按当前权威条件独立校验；重连、刷新和跨入口查看不能产生第二次世界效果。

## 5. 叶级产品要求与验收

<a id="req-agent-surface-001"></a>
### REQ-AGENT-SURFACE-001：跨 surface 保留同一决策锚点

- 要求：在受支持 viewport、语言和连接状态下，玩家必须能找到当前目标、主要 blocker、最近可信反馈和下一决策或恢复入口；呈现变化不得改写权限或世界结果。
- 验收：AC-AGENT-SURFACE-001

<a id="ac-agent-surface-001"></a>
### AC-AGENT-SURFACE-001：窄屏、fallback 与恢复状态仍可读

- 覆盖要求：REQ-AGENT-SURFACE-001
- 场景与结果：代表性 desktop、窄屏、低高度和双语 fallback 样例中，四个决策锚点仍可辨认；中断、恢复中、恢复失败和已接受结果保持不同语义，并提供真实下一步。
- 证据边界：只证明玩家可读语义和不误导边界；具体布局、控件、locale 资源和连接字段由 Viewer/WASM/QA authority 验证。

<a id="req-agent-surface-002"></a>
### REQ-AGENT-SURFACE-002：不支持行动不得伪装成传输失败

- 要求：当当前模式不支持某行动、权限/规则阻塞或请求已被接受时，surface 必须与连接中断和未确认请求区分，并给出适用的替代、等待、返回或重新验证路径。
- 验收：AC-AGENT-SURFACE-002

<a id="ac-agent-surface-002"></a>
### AC-AGENT-SURFACE-002：重连不会创造能力或世界效果

- 覆盖要求：REQ-AGENT-SURFACE-002
- 场景与结果：不支持行动、权限阻塞、已接受未结算请求和断连同时出现时，首要解释按当前权威语义呈现；重连、刷新、后台 tick 或跨入口查看不会把请求变成成功、取消或第二次世界效果。
- 证据边界：请求 acceptance、世界结果、去重与恢复合同由 runtime/Agent 专业 authority 提供；产品层不冻结 transport 或 UI 实现。

## 5.1 组合验收

- PSC-1：代表性 desktop、窄屏和低高度 viewport 中，当前目标、主要 blocker、接受/结果反馈与下一步保持可读且可恢复。
- PSC-2：信息密度或 panel 可见性改变后，玩家仍能找到主要决策面；隐藏次级内容不会制造权限、结果或世界事实变化。
- PSC-3：受支持语言及其 fallback 中，关键意图、状态、错误和恢复语义等价，语言选择不改变权威结果。
- PSC-4：断连与恢复样例明确区分连接状态、request acceptance 与权威进展；恢复或后台活动不代签成功。
- PSC-5：证据来自当前 Viewer、runtime 与 QA 专业权威；历史 EGUI/Bevy/Web 完成记录、单张截图或本地 fallback 不能单独成立产品结论。
- PSC-6：代表性模式转换或不支持行动样例能区分能力边界、断连、权限/规则阻塞与已接受请求，并提供真实替代或安全返回；自动推进或接口计数不代签权威世界结果。
- PSC-7：代表性重叠状态（已接受未结算+断连/恢复中、未接受+权威不可用、不支持行动+断连、已拒绝+断连）按 4.1 的主语义显示唯一首要解释和可执行下一步；重连、刷新或跨入口重试不覆盖已确认结果、不把 pending 变成成功/失败/取消，也不产生第二次世界效果。

## 5.2 专业 owner、authority 与测试层级追踪

本表是产品语义到专业合同和验证证据的导航，不是当前实现、通过结论或发布/readiness 声明。每个 owner 只负责表中列出的边界；证据必须由对应专业 authority 在适用的任务、测试和 artifact 中产生。

| REQ / AC | 专业 owner 与真实边界 | 专业域 PRD-ID | 专业权威（可导航） | 验证证据（应提供，不预示当前结果） | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| [REQ-AGENT-SURFACE-001](#req-agent-surface-001) / [AC-AGENT-SURFACE-001](#ac-agent-surface-001) | `producer_system_designer`：产品语义、非目标和组合验收，不拥有专业实现或 readiness verdict；`viewer_engineer`：viewport、fallback 文本、连接状态和下一步在正式 surface 的可读表达，不拥有世界真值；`runtime_engineer`：committed/pending/recovery 的权威结果与无第二次效果，不拥有布局；`wasm_platform_engineer`：WASM bridge/executor 的 ABI、限制、结构化失败和兼容边界，不拥有 locale 或布局；`agent_engineer`：Agent observation/action、provider 失败反馈和模式边界，不授予 world acceptance；`qa_engineer`：跨 surface 的可复验证据与失败签名，不以单一局部 green 代签产品结论。 | `PRD-WORLD_SIMULATOR-001/039/041/046`；`PRD-WORLD_RUNTIME-001/031/033`；`PRD-WORLD_RUNTIME-002/036/037`；`PRD-WORLD_SIMULATOR-040/040B`；`PRD-TESTING-003` / `PRD-TESTING-WEB-001/002/003` | [`world-simulator active baseline`](../../world-simulator/prd.md#active-requirement-baseline)、[`Viewer 证据边界`](../../world-simulator/viewer/viewer-manual.manual.md#证据边界)、[`Viewer 连接与 no-throw 合同`](../../world-simulator/viewer/viewer-web-semantic-test-api.prd.md#current-connection-and-no-throw-boundary)、[`runtime receipt 边界`](../../world-runtime/prd.md#executionreceipt、tick-compatibility-与-finality-边界)、[`WASM sandbox 与工件完整性`](../../world-runtime/wasm/wasm-executor.prd.md#sandbox-与工件完整性契约)、[`Agent 双模式验证`](../../world-simulator/llm/provider-agent-dual-mode.prd.md#6-validation-decision-record)、[`testing validation`](../../testing/prd.md#6-validation-decision-record) | `Viewer`：headed S6 desktop、窄屏、低高度和双语 fallback 样例，保存 screenshot、console、snapshot/state 与实际操作结果，检查目标、blocker、可信反馈和恢复入口；`runtime`：同一 fixture 对账 committed receipt、pending/recovery、canonical state/journal/replay root；`WASM`：ABI/limit/capability rejection、工件 hash、结构化错误和 replay/no-effect 证据；`Agent`：`player_parity`/`headless_agent` 的同场景 observation/action、mode metadata 与 provider 失败到 Wait/拒绝的证据；`QA`：把上述输入合并为可复核的跨 surface 证据包，并记录未覆盖 viewport、语言、连接状态和专业边界。 | `test_tier_required`：产品/authority 链接治理、Viewer/Agent/runtime/WASM 定向合同和最小 S6 required 输入；`test_tier_full`：跨 surface 的双语、窄屏/低高度、恢复与 replay 对账，含 Agent parity、WASM 兼容/失败路径和 QA 组合审查。 |
| [REQ-AGENT-SURFACE-002](#req-agent-surface-002) / [AC-AGENT-SURFACE-002](#ac-agent-surface-002) | `producer_system_designer`：产品语义、非目标和组合验收，不拥有专业实现或 readiness verdict；`viewer_engineer`：把能力边界、权限/规则阻塞、accepted/pending 和断连表达为不同可读状态，不制造 action 或 world effect；`runtime_engineer`：acceptance、commit、拒绝、去重、恢复和 canonical world effect 的唯一顺序；`wasm_platform_engineer`：模块调用的 deterministic limits、compatibility fault 和失败时无副作用边界；`agent_engineer`：Agent/provider 的 action contract、Wait/拒绝与恢复反馈，不把 provider 建议当作权威应用；`qa_engineer`：覆盖重叠状态和负例的 required/full 证据，不把 reconnect/refresh 当成功。 | `PRD-WORLD_SIMULATOR-039/041/046`；`PRD-WORLD_RUNTIME-001/031/033/047`；`PRD-WORLD_RUNTIME-002/036/037`；`PRD-WORLD_SIMULATOR-040/040B`；`PRD-TESTING-003` / `PRD-TESTING-WEB-001/002/003` | [`runtime snapshot/replay contract`](../../world-runtime/prd.md#snapshot、replay、version-与-canonical-timeline-兼容)、[`Agent authority/capability boundary`](../../world-runtime/runtime/agent-cognition-lifecycle.prd.md#6-agentintentv2-与-authoritycapability-边界)、[`Agent deterministic/replay acceptance`](../../world-runtime/runtime/agent-cognition-lifecycle.prd.md#8-acceptance-与-deterministicreplay-verification)、[`WASM sandbox 与工件完整性`](../../world-runtime/wasm/wasm-executor.prd.md#sandbox-与工件完整性契约)、[`Agent 双模式验证`](../../world-simulator/llm/provider-agent-dual-mode.prd.md#6-validation-decision-record)、[`Web UI 验证决策记录`](../../testing/manual/web-ui-agent-browser-closure-manual.prd.md#6-validation-decision-record)、[`testing validation`](../../testing/prd.md#6-validation-decision-record) | `Viewer`：不支持行动、权限/规则阻塞、accepted 未结算、未确认和断连重叠样例的 headed S6 状态/操作证据；`runtime`：fresh/stale/duplicate/conflict、accepted-but-uncommitted、reconnect/replay/restart fixture 的 disposition、receipt/state root 和 exactly-once 对账；`WASM`：超时、超限、坏工件、compatibility fault 与恢复后的 no-effect/结构化错误证据；`Agent`：相同 observation/action contract 下 provider failure、Wait/拒绝、重试和 replay 不产生隐式替代行动或第二次效果；`QA`：按状态优先级核对首要解释、真实下一步与未覆盖组合，明确 required 与 full 的证据窗口。 | `test_tier_required`：状态分类、能力边界、拒绝/Wait/no-effect 与玩家可读下一步的定向合同及 S6 required 输入；`test_tier_full`：四类重叠状态的恢复、replay、去重、跨入口/跨 adapter 组合证据，以及 Viewer/runtime/WASM/Agent/QA 的同候选对账。 |

## 6. 范围与非目标

覆盖可用空间、信息密度、当前模式支持的行动、语言、关键文本、连接中断与恢复的跨 surface 产品连续性。不定义具体 panel/module、fullscreen 控件、布局宽度、hit boundary、字体/资产、local JSON/cache、控制 profile、时间/事件计数、WebSocket 时序、Test API、测试命令或发布/readiness claim。

## 7. 接口 / 数据

产品层只定义 `决策锚点 -> surface 状态变化 -> 权威反馈 -> 恢复入口` 的可读语义。viewport、locale、connection、request、ack、transport 和测试字段由 Viewer、runtime 与 testing 专业 authority 定义。

## 8. 里程碑

1. 维持稳定且可从模块入口到达的 PRD 与 design。
2. 吸收并删除历史 panel、declutter、fullscreen、i18n、Web usability 与 step acknowledgement 碎片文档。
3. 当前 Viewer/runtime/testing authority 持续提供实现和验证证据。

## 9. 风险

- 将旧 EGUI/Web 控件误写成当前能力。
- 将连接恢复或 request acceptance 误写成世界已经推进。
- 将语言、viewport 或本地偏好实现细节冻结为产品合同。
- 删除仍承担协议真值的专业文档；本专题只允许删除已有代码、测试或当前专业文档承接的历史来源。
