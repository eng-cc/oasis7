# oasis7 产品模块入口

本页是产品信息架构的唯一总入口。它按玩家价值组织产品，不替代 `doc/README.md` 的工程模块矩阵，也不改变各专业域 PRD 的实现与验收权威。

## 四大产品模块

| 产品模块 | 唯一入口 | 产品职责 |
| --- | --- | --- |
| 世界规则与核心玩法 | [`doc/game/prd.md`](../game/prd.md) | 定义玩家目标、核心循环、成长、资源压力与世界规则体验。 |
| 大世界基础设施 | [`doc/product/world-infrastructure/prd.md`](world-infrastructure/prd.md) | 统一玩家可建设区域设施与持久、可审计、可扩展的大世界状态底座。 |
| 智能体与世界模拟 | [`doc/world-simulator/prd.md`](../world-simulator/prd.md) | 把场景、Agent/LLM 决策、世界状态与可交互模拟体验连接起来。 |
| 玩家入口与发行 | [`README.md`](../../README.md) | 统一玩家如何了解、进入、安装和验证当前可用产品体验及其公开边界。 |

产品层只定义产品承诺、组合关系和跨域验收。工程实现、专题契约与任务状态继续由对应模块 PRD、design、project 和 GitHub task issue evidence 承载。

