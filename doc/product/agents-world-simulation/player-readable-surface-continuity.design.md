# 玩家可读表面连续性产品设计

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/player-readable-surface-continuity.prd.md`](player-readable-surface-continuity.prd.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`

本文定义跨 surface 的稳定设计语义，不把历史 Viewer 布局或控件提升为当前实现要求。

## 1. 稳定阅读锚点

任何受支持布局都优先保留四个锚点：当前目标、主要 blocker、最近可信行动反馈、下一决策或恢复入口。次级世界信息、设置和诊断可以重排或渐进隐藏，但不能抢占或切断这四个锚点。

## 2. 响应式与遮挡

- desktop、窄屏和低高度可以采用不同结构；主要行动表面不被不可恢复的 panel 或 overlay 遮挡。
- 收起或隐藏是可理解、可逆且可发现的状态，不依赖记住历史控件位置。
- 全屏或密度偏好只调整呈现，不改变世界真值、权限、选择或动作结果。
- 颜色、动效、hover 和屏幕位置不能成为关键状态的唯一信号。

## 3. 双语与 fallback

- 语言选择及当前语言状态可辨认，切换不会改变动作或世界状态。
- 目标、blocker、接受/拒绝、结果、错误和恢复文本优先保证语义覆盖。
- 缺失翻译时使用可读 fallback 并保留诊断线索；不得显示空白或把技术键当成已完成本地化。

## 4. 连接状态

- `已连接 / 中断 / 恢复中 / 恢复失败` 使用文本或等价语义区分。
- 恢复入口与当前决策上下文相邻；重连成功只证明连接恢复，不证明请求、动作或世界进展成功。
- 当前 surface authority 未支持的 retry、fullscreen、panel 或 locale 动作不得展示成可用能力。

## 5. 当前模式与行动反馈

- 在主动作附近表达当前可用行动；不支持、连接中断、规则/权限阻塞和已接受请求必须保持不同含义，不能共用为“失败”或“请重连”。
- 不支持的行动提供当前模式适用的替代、等待或安全返回入口；能力边界不因布局、自动开始、重载或背景推进而消失。
- 界面时间、事件计数和动画只帮助阅读；只有权威反馈能够确认行动产生世界后果。没有可见进展时，继续呈现当前可信状态和下一决策，而不是用自动刷新制造成功印象。

## 6. 非承诺

- 不承诺旧 EGUI 右栏、模块可见性缓存、专用 fullscreen toggle、固定断点或历史 Web 布局。
- 不定义 control profile、动作枚举、seek/step、逻辑时间/事件游标、ack 字段、字体、翻译键、localStorage/JSON 路径、WebSocket callback/backoff、viewport hit test 或 Test API。
- 不以历史截图、旧测试、mock、software-safe 或局部 DOM 存在代签当前产品能力。
