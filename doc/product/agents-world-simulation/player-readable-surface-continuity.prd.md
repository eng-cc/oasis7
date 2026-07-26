# 玩家可读表面的连续性

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/player-readable-surface-continuity.design.md`](player-readable-surface-continuity.design.md)
- 产品迁移追踪：[`doc/product/agents-world-simulation/player-readable-surface-continuity.project.md`](player-readable-surface-continuity.project.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md)、[`world-simulator PRD`](../../world-simulator/prd.md)、[`Web 语义测试 API`](../../world-simulator/viewer/viewer-web-semantic-test-api.prd.md)

本文定义正式玩家表面在 viewport、信息密度、语言与连接状态变化时保持可理解、可操作和可恢复的长期产品承诺。它不冻结 Viewer 布局、组件、Web/EGUI/Bevy 实现、协议字段、缓存、字体资产或当前发布结论。

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

## 5. 组合验收

- PSC-1：代表性 desktop、窄屏和低高度 viewport 中，当前目标、主要 blocker、接受/结果反馈与下一步保持可读且可恢复。
- PSC-2：信息密度或 panel 可见性改变后，玩家仍能找到主要决策面；隐藏次级内容不会制造权限、结果或世界事实变化。
- PSC-3：受支持语言及其 fallback 中，关键意图、状态、错误和恢复语义等价，语言选择不改变权威结果。
- PSC-4：断连与恢复样例明确区分连接状态、request acceptance 与权威进展；恢复或后台活动不代签成功。
- PSC-5：证据来自当前 Viewer、runtime 与 QA 专业权威；历史 EGUI/Bevy/Web 完成记录、单张截图或本地 fallback 不能单独成立产品结论。
- PSC-6：代表性模式转换或不支持行动样例能区分能力边界、断连、权限/规则阻塞与已接受请求，并提供真实替代或安全返回；自动推进或接口计数不代签权威世界结果。

## 6. 范围与非目标

覆盖可用空间、信息密度、当前模式支持的行动、语言、关键文本、连接中断与恢复的跨 surface 产品连续性。不定义具体 panel/module、fullscreen 控件、布局宽度、hit boundary、字体/资产、local JSON/cache、控制 profile、时间/事件计数、WebSocket 时序、Test API、测试命令或发布/readiness claim。

## 7. 接口 / 数据

产品层只定义 `决策锚点 -> surface 状态变化 -> 权威反馈 -> 恢复入口` 的可读语义。viewport、locale、connection、request、ack、transport 和测试字段由 Viewer、runtime 与 testing 专业 authority 定义。

## 8. 里程碑

1. 建立稳定 PRD、design 与迁移 project。
2. 吸收并删除历史 panel、declutter、fullscreen、i18n、Web usability 与 step acknowledgement 碎片文档。
3. 当前 Viewer/runtime/testing authority 持续提供实现和验证证据。

## 9. 风险

- 将旧 EGUI/Web 控件误写成当前能力。
- 将连接恢复或 request acceptance 误写成世界已经推进。
- 将语言、viewport 或本地偏好实现细节冻结为产品合同。
- 删除仍承担协议真值的专业文档；本专题只允许删除已有代码、测试或当前专业文档承接的历史来源。
