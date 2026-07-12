# testing/governance 专题入口

本目录收拢 testing 的质量门禁、证据层级、playability 评审与测试治理专题。它只负责按问题分流和声明专题 authority；不要把专题规则、操作步骤或历史结论再复制到模块根入口。

## 首读路径

| 要回答的问题 | Canonical 入口 | Authority 边界 |
| --- | --- | --- |
| 自动化、agent、外部信号各能证明什么；何时可以声称“好玩” | `playability-evidence-stack-2026-05-06.prd.md` | 跨层证据栈、`go/watch/hold/block` 组合规则与高层 claim 边界 |
| `L4A synthetic`、`L4B embodied agent`、`L5` 如何区分 | `playability-l4-synthetic-human-split-2026-05-06.prd.md` | L4 分层与可选内部真人校准边界 |
| 标准角色 subagent 怎么组成内部评审 | `playability-subagent-review-system-2026-05-06.prd.md` | role review packet、触发矩阵、汇总与 stop conditions |
| 多种玩家风格如何作为内部输入，而不新增正式角色 | `playability-simulated-player-persona-panel-2026-05-06.prd.md` | persona panel 的输入卡、升级边界与 role-review handoff |
| release-gate 指标、质量趋势、审计检查或确定性 guard | `doc/testing/prd.index.md` 的 `governance/` 专题表 | 各专项 topic 的精确文件检索；不替代以上 playability authority |

## 与相邻入口的职责

- `doc/testing/README.md`：testing 模块首读与子域选择；涉及 testing governance 时只路由到本页。
- `doc/testing/prd.index.md`：文件级精确检索和三件套可达性；本页不复制其完整长表。
- `doc/testing/prd.md` / `project.md`：模块测试门禁基线与当前执行状态；本页不承载模块状态。
- `testing-manual.md` 与 `doc/testing/manual/*.manual.md`：operator 操作步骤；本页只把需要操作的读者导向对应 manual，不重述步骤。

## 维护规则

- 新增或退役 `governance/` 专题时：更新 `doc/testing/prd.index.md` 的文件级可达性；若其改变首读问题或 canonical authority，同时更新本页的一行分流。
- 专题正文、triplet 配对、证据和历史留痕仍留在原路径；本页不是新的规则正文或状态汇总面。
- `L4A`、`L4B`、`L5` 的语义、角色越权边界与 release/claim 结论只以各自 canonical topic 为准，导航文字不得覆盖它们。
