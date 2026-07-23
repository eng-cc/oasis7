# oasis7 产品模块入口

本页是产品信息架构的唯一总入口。它按玩家价值组织产品，不完全替代 `doc/README.md` 的工程模块矩阵；四大模块的产品真值与设计将逐步迁移到本目录下的文档树，散落在外部各专业模块的相关产品文档也将随语义归拢逐步清理、删除。

本目录的文档树以 PRD 为主，并可按需包含同名配对的 design 和 project：PRD 承载产品真值、产品承诺、组合关系和跨域验收；design 承载产品设计；project 仅承载产品层面的迁移和交付追踪，不复制实现计划、测试步骤、运行处置或任务状态。

## 四大产品模块

| 产品模块 | 唯一入口 | 产品职责 |
| --- | --- | --- |
| 世界规则与核心玩法 | [`doc/product/world-rules-core-gameplay/prd.md`](world-rules-core-gameplay/prd.md) | 定义玩家目标、核心循环、成长、资源压力与世界规则体验。 |
| 大世界基础设施 | [`doc/product/world-infrastructure/prd.md`](world-infrastructure/prd.md) | 统一玩家可建设区域设施与持久、可审计、可扩展的大世界状态底座。 |
| 智能体与世界模拟 | [`doc/product/agents-world-simulation/prd.md`](agents-world-simulation/prd.md) | 把场景、Agent/LLM 决策、世界状态与可交互模拟体验连接起来。 |
| 玩家入口与发行 | [`doc/product/player-entry-distribution/prd.md`](player-entry-distribution/prd.md) | 统一玩家如何了解、进入、安装和验证当前有证据支持的技术预览及其公开边界。 |

每个产品模块以主 PRD 为权威入口，但不限于单个文件；可以按长期稳定的产品主题建立专题分册，形成“模块入口 → 主 PRD → 专题分册”的文档树。专题分册必须由模块入口可达并回链主 PRD，不得按日期或短期小功能拆成设计碎片。

迁移按文件逐个进行：先判定其中哪些内容属于产品承诺、产品设计、跨域组合或端到端验收，并将这些语义回填到对应产品模块；专业规则、实现合同、技术 How、工程实现、测试/运维和任务证据仍留在专业域并由产品文档链接。迁移治理应以“语义完整归位并删除源文件”为默认目标，而不是只新增产品文档、继续保留重复或日期化的旧入口。迁移前后均须保留模块入口与专题回链，修复所有活跃引用；仅当产品语义已完整回填、专业域权威未丢失且活跃引用已修复时，才可删除原文件。若源文件仍承载尚未迁移的专业真值而必须暂时保留，须明确记录剩余语义、目标权威与后续删除条件，将其作为迁移债务继续治理。完成条件是读者可从四大模块入口到达产品真值，并能继续下钻到对应专业域权威和验证证据。

如果需要从产品承诺继续下钻到规则、实现契约或验证证据，按下表进入专业域权威：

- 世界规则与核心玩法 → [`doc/game/prd.md`](../game/prd.md)
- 大世界基础设施 → [`doc/game/prd.md`](../game/prd.md)、[`doc/world-runtime/prd.md`](../world-runtime/prd.md)、[`doc/p2p/prd.md`](../p2p/prd.md)
- 智能体与世界模拟 → [`doc/world-simulator/prd.md`](../world-simulator/prd.md)
- 玩家入口与发行 → [根 `README.md`](../../README.md)、[`doc/world-simulator/prd.md`](../world-simulator/prd.md)

专业规则、实现合同、专业 PRD-ID 和测试机制由对应专业模块的 PRD 承载；技术 How 与工程实现由 design 承载；测试/运维由对应专业文档承载；任务、状态与过程证据由各专业模块的 project.md 和 GitHub task issue evidence 承载。
