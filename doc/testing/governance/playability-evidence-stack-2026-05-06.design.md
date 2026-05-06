# oasis7: 好玩性证据栈（2026-05-06）设计

- 对应需求文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
- 对应项目管理文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md`

审计轮次: 1

## 1. 设计定位
把 testing 模块里与“是否好玩”相关的治理口径统一成一个解释层专题，明确证据层级、证明边界、组合规则和现有脚本/文档锚点。

## 2. 设计结构
- 证据层:
  - L1 自动化基线
  - L2 Agent/fixture probe
  - L3 遥测与实验
  - L4 结构化真人试玩
  - L5 受控外部信号
- 组合层:
  - `block / hold / watch / go`
- 映射层:
  - 把 `software_safe`、`pure_api`、`--no-llm`、playability card、`run-producer-playtest.sh`、limited preview 等现有入口挂到具体层级。

## 3. 关键接口 / 入口
- 专题 PRD: `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
- 专题 Project: `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md`
- 模块根入口: `doc/testing/prd.md`、`doc/testing/project.md`
- 操作手册入口: `testing-manual.md`
- 真人试玩 / 玩法结果入口: `doc/playability_test_result/*`

## 4. 约束与边界
- 不新增 SDK、服务端遥测管线或实验平台实现。
- 不重写现有 trust gate / capability gate 证据，只统一它们在更大证据栈中的位置。
- 不允许低层证据替代高层证据作出更强 claim。

## 5. 设计演进计划
- 先建立专题和根入口互链。
- 再把正式 evidence packet 的分层结论逐步补齐。
- 后续若引入真实实验/遥测系统，也只作为 L3 能力扩展，不改变 L4/L5 的必要性。
