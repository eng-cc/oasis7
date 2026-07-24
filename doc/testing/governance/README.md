# testing/governance 专题入口

本目录收拢 testing 的质量门禁、证据层级、playability 评审与测试治理专题。它只负责按问题分流和声明专题 authority；不要把专题规则、操作步骤或历史结论再复制到模块根入口。

## 首读路径

| 要回答的问题 | Canonical 入口 | Authority 边界 |
| --- | --- | --- |
| 自动化、agent、外部信号各能证明什么；何时可以声称“好玩” | [`../prd.md`](../prd.md) 的 Durable Playability Evidence Governance | 跨层证据栈、`go/watch/hold/block` 组合规则与高层 claim 边界 |
| `L4A synthetic`、`L4B embodied agent`、`L5` 如何区分，以及如何实际执行 | [`../../../testing-manual.md`](../../../testing-manual.md) 的 L4A/L4B/L5 分层 | operator 入口、可选内部真人校准与非替代边界 |
| 标准角色与 persona 如何组成内部评审 | [`../prd.md`](../prd.md) 的 Durable Playability Evidence Governance | role/persona card contract、触发、hand-off 与 stop conditions；persona 不是正式角色 |
| 质量趋势指标如何定义；当前窗口的样本和结论在哪里 | `testing-quality-trend-tracking-2026-03-11.prd.md` 定义口径；`../evidence/testing-quality-trend-baseline-2026-03-11.md` 保存报告 | PRD 负责公式、阈值与采集边界；evidence 负责可更新的窗口事实，不另设角色 handoff 文档 |
| release-gate 指标、质量趋势、审计检查或确定性 guard | `doc/testing/prd.index.md` 的 `governance/` 专题表 | 各专项 topic 的精确文件检索；不替代以上 playability authority |

## 与相邻入口的职责

- `doc/testing/README.md`：testing 模块首读与子域选择；涉及 testing governance 时只路由到本页。
- `doc/testing/prd.index.md`：文件级精确检索和三件套可达性；本页不复制其完整长表。
- `doc/testing/prd.md` / `project.md`：模块测试门禁基线与当前执行状态；本页不承载模块状态。
- `testing-manual.md` 与 `doc/testing/manual/*.manual.md`：operator 操作步骤；本页只把需要操作的读者导向对应 manual，不重述步骤。

## 维护规则

- 新增或退役 `governance/` 专题时：更新 `doc/testing/prd.index.md` 的文件级可达性；若其改变首读问题或 canonical authority，同时更新本页的一行分流。
- 专题正文、证据和历史留痕仍留在其专业 authority、Git history 与 GitHub task evidence；本页不是新的规则正文或状态汇总面。
- `L4A`、`L4B`、`L5` 的语义、角色越权边界与 release/claim 结论以 `doc/testing/prd.md` 和 `testing-manual.md` 为准，导航文字不得覆盖它们。
