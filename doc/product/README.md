# oasis7 产品模块入口

本页是产品信息架构的唯一总入口。它按玩家价值组织产品，不完全替代 `doc/README.md` 的工程模块矩阵；四大模块的产品真值与设计将逐步迁移到本目录下的文档树，散落在外部各专业模块的相关产品文档也将随语义归拢逐步清理、删除。

本目录的文档树以 PRD 为主，并可按需包含同名配对的 design：PRD 承载产品真值、产品承诺、组合关系和跨域验收；design 承载产品设计。迁移与交付追踪、任务状态和过程证据只由 GitHub Issue / GitHub Project-backed task truth 承载，不创建本地 project ledger。

## 四大产品模块

| 产品模块 | 唯一入口 | 产品职责 |
| --- | --- | --- |
| 世界规则与核心玩法 | [`doc/product/world-rules-core-gameplay/prd.md`](world-rules-core-gameplay/prd.md) | 定义玩家目标、核心循环、成长、资源压力与世界规则体验。 |
| 大世界基础设施 | [`doc/product/world-infrastructure/prd.md`](world-infrastructure/prd.md) | 区块链/分布式系统与确定性世界运行时底座：最终性、权威状态、复制、存储、网络、恢复和版本化执行边界。 |
| 智能体与世界模拟 | [`doc/product/agents-world-simulation/prd.md`](agents-world-simulation/prd.md) | 把场景、Agent/LLM 决策、世界状态与可交互模拟体验连接起来。 |
| 玩家入口与发行 | [`doc/product/player-entry-distribution/prd.md`](player-entry-distribution/prd.md) | 统一玩家如何了解、进入、安装和验证当前有证据支持的技术预览及其公开边界。 |

每个产品模块以主 PRD 为权威入口，但不限于单个文件；可以按长期稳定的产品主题建立专题分册，形成“模块入口 → 主 PRD → 专题分册”的文档树。专题分册必须由模块入口可达并回链主 PRD，不得按日期或短期小功能拆成设计碎片。

迁移按文件逐个进行：先判定其中哪些内容属于产品承诺、产品设计、跨域组合或端到端验收，并将这些语义回填到对应产品模块；专业规则、实现合同、技术 How、工程实现、测试/运维和任务证据仍留在专业域并由产品文档链接。迁移治理应以“语义完整归位并删除源文件”为默认目标，而不是只新增产品文档、继续保留重复或日期化的旧入口。迁移前后均须保留模块入口与专题回链，修复所有活跃引用；仅当产品语义已完整回填、专业域权威未丢失且活跃引用已修复时，才可删除原文件。若源文件仍承载尚未迁移的专业真值而必须暂时保留，须明确记录剩余语义、目标权威与后续删除条件，将其作为迁移债务继续治理。完成条件是读者可从四大模块入口到达产品真值，并能继续下钻到对应专业域权威和验证证据。

## 产品树完整性与迁移验收

本节是产品层自身的结构合同；它约束产品语义如何归位，不替代专业域规则、实现、测试或任务生命周期。

### 结构规则与边界

- `doc/product/README.md` 是唯一产品入口，且只承认上表四个模块。`core`、`game`、`world-runtime`、`world-simulator`、`p2p`、`testing`、`engineering` 与 `site` 是专业或治理域，不得通过新增产品入口变成第五个产品模块。
- 每个模块根 PRD 负责该模块的产品承诺、组合关系、Non-Goals 与跨域验收；长期专题必须由模块根 PRD 的“活跃产品专题”可达并回链根 PRD。专题不得声明模块根的保留身份元数据或另立产品总入口。
- 稳定的玩家/产品语义进入对应产品模块；专业规则、实现合同、技术 How、工程实现、测试/运维和任务证据留在其专业权威，产品文档只链接并说明组合边界。产品文档不得复制可变任务状态、review ledger 或发布证据。
- `superseded` / `retired` 文档是迁移债务或历史引用，不是 active authority、路线图或验收入口。源文件只有在语义完整回填、专业权威仍可达、活跃引用修复后才能删除；暂存文件必须记录剩余语义、接收 owner 与删除条件，不能以重复入口长期替代迁移。

### Done：可验证验收

- **PD-1 四模块闭合**：入口表恰好列出四个模块根 PRD，四个根 PRD 均能回链本页；不存在由别的目录或日期化文件承载的第五个产品入口。
- **PD-2 专题可达**：每个 `active` 专题都能从所属根 PRD 的“活跃产品专题”到达并回链；专题只声明自身生命周期与专业权威，不伪造模块根身份。
- **PD-3 迁移可判定**：每个被吸收的来源语义都能在接收模块或其明确链接的专业权威中定位；尚未完成的来源保留 `superseded` / `retired`、剩余语义、目标 owner 和删除条件，不能继续作为 active authority。
- **PD-4 权威边界可追踪**：模块根 PRD 的每条成功标准都能追踪到 owner、专业 PRD-ID、权威文档、验证证据和测试层级；产品层不把局部专业 green 或任务状态当作产品完成。
- **PD-5 检查边界诚实**：对产品树运行 `./scripts/doc-governance-check.sh`、`python3 scripts/product-doc-governance-check.test.py` 必须通过，且 `git diff --check` 无输出；这些自动检查只证明已枚举的结构合同（四行入口清单、根 PRD 身份/生命周期/声明的 authority backlink、专题声明、成功标准—追踪行、保留元数据/ledger/path/行数约束）成立，不证明语义迁移完整、所有通用专业 authority 链接或根 Markdown backlink 完整，也不证明 PD-1–PD-4 的语义内容和 traceability 值正确；后者必须由人工/对应专业角色复核。

### Non-Goals

- 不在本页定义玩法数值、runtime/WASM/Agent/Viewer 实现、测试步骤、运维 runbook、渠道文案或发布 verdict。
- 不要求一次性删除所有历史或专业来源；未完成迁移的语义应按上述迁移债务规则保留，直到接收与引用修复可被验证。
- 不把本页的结构检查当作产品可玩、发布就绪或线上安全证明；这些结论仍由对应专业 authority、QA 与公开 claim 边界共同决定。

如果需要从产品承诺继续下钻到规则、实现契约或验证证据，按下表进入专业域权威：

- 世界规则与核心玩法 → [`doc/game/prd.md`](../game/prd.md)
- 大世界基础设施 → [`doc/p2p/prd.md`](../p2p/prd.md)、[`doc/world-runtime/prd.md`](../world-runtime/prd.md)、[`doc/testing/prd.md`](../testing/prd.md)
- 智能体与世界模拟 → [`doc/world-simulator/prd.md`](../world-simulator/prd.md)
- 玩家入口与发行 → [根 `README.md`](../../README.md)、[`doc/world-simulator/prd.md`](../world-simulator/prd.md)

专业规则、实现合同、专业 PRD-ID 和测试机制由对应专业模块的 PRD 承载；技术 How 与工程实现由 design 承载；测试/运维由对应专业文档承载；任务、状态与过程证据只由 GitHub task issue evidence 承载。
