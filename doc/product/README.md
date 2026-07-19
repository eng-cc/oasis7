# oasis7 产品模块入口

本页是产品信息架构的唯一总入口。它按玩家价值组织产品，不替代 `doc/README.md` 的工程模块矩阵，也不改变各专业域 PRD 的实现与验收权威。

## 四大产品模块

| 产品模块 | 唯一入口 | 产品职责 |
| --- | --- | --- |
| 世界规则与核心玩法 | [`doc/product/world-rules-core-gameplay/prd.md`](world-rules-core-gameplay/prd.md) | 定义玩家目标、核心循环、成长、资源压力与世界规则体验。 |
| 大世界基础设施 | [`doc/product/world-infrastructure/prd.md`](world-infrastructure/prd.md) | 统一玩家可建设区域设施与持久、可审计、可扩展的大世界状态底座。 |
| 智能体与世界模拟 | [`doc/product/agents-world-simulation/prd.md`](agents-world-simulation/prd.md) | 把场景、Agent/LLM 决策、世界状态与可交互模拟体验连接起来。 |
| 玩家入口与发行 | [`doc/product/player-entry-distribution/prd.md`](player-entry-distribution/prd.md) | 统一玩家如何了解、进入、安装和验证当前有证据支持的技术预览及其公开边界。 |

每个产品模块以主 PRD 为权威入口，但不限于单个文件；可以按长期稳定的产品主题建立专题分册，形成“模块入口 → 主 PRD → 专题分册”的文档树。专题分册必须由模块入口可达并回链主 PRD，不得按日期或短期小功能拆成设计碎片。

如果需要从产品承诺继续下钻到规则、实现契约或验证证据，按下表进入专业域权威：

- 世界规则与核心玩法 → [`doc/game/prd.md`](../game/prd.md)
- 大世界基础设施 → [`doc/game/prd.md`](../game/prd.md)、[`doc/world-runtime/prd.md`](../world-runtime/prd.md)、[`doc/p2p/prd.md`](../p2p/prd.md)
- 智能体与世界模拟 → [`doc/world-simulator/prd.md`](../world-simulator/prd.md)
- 玩家入口与发行 → [根 `README.md`](../../README.md)、[`doc/world-simulator/prd.md`](../world-simulator/prd.md)

产品层只承载产品承诺、产品设计、组合关系和跨域验收。专业规则、技术 How、工程实现、测试/运维与任务状态继续由对应专业模块的 PRD、design、project 和 GitHub task issue evidence 承载。
