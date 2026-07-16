# engineering/governance 运行治理入口

本目录收纳工程治理的运行型资料。本页只做按问题分流；各专题正文仍是各自事实、检查步骤与状态的权威来源。

## 从这里开始

- 想确认项目 `local` / `test` / `production` 环境边界、云上服务清单、`public_testnet` 与 `mainnet` 的声明限制：`environment-lanes-and-inventory-2026-05-29.md`
- 想执行一次人工触发的仓库健康巡检、选择检查范围或判定 findings 的归属：`repository-health-manual-inspection.runbook.md`
- 想进行季度工程治理复核、记录趋势或 remediation owner：使用 `repository-health-manual-inspection.runbook.md` 的“Quarterly Review”记录结构，趋势基线见 `../evidence/engineering-governance-trend-baseline-2026-03-11.md`

## 边界

- 文档树结构、README 职责与 redirect 规则由 `../doc-governance/README.md` 分流；本页不复述这些共享规则。
- 当前 task truth、证据 sink、角色派工和 PR 主链规则由 `../workflow/source-of-truth.md` 定义；运行型资料不得取代它。
- 新增本目录的运行治理文档时，同批更新本页；只在需要文件级三件套检索时更新 `../prd.index.md`。
