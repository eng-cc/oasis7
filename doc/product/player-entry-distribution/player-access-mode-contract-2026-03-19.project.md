# oasis7：玩家访问模式产品契约追踪（2026-03-19）

- 父模块 PRD: [`玩家入口与发行 PRD`](prd.md)
- 产品模块总入口: [`doc/product/README.md`](../README.md)
- 对应产品专题 PRD: [`doc/product/player-entry-distribution/player-access-mode-contract-2026-03-19.prd.md`](player-access-mode-contract-2026-03-19.prd.md)
- 对应产品专题设计: [`玩家访问模式产品契约设计`](player-access-mode-contract-2026-03-19.design.md)
- 专业域权威: [`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

## 产品专题追踪边界

本文件只保留产品 taxonomy 与 claim 边界的迁移和可追溯性；不维护实现任务、状态、测试命令或发布放行结论。`viewer` / `pure_api` 的具体 Viewer、provider、Launcher、runtime 与测试合同继续由下层专业域 PRD、design、project 及 GitHub task issue evidence comments 承载。

## 任务拆解

产品层不维护本专题的本地任务清单。实现与验证工作由绑定 GitHub task issue evidence comments 和专业模块 `project.md` 拆解；本专题只在产品 taxonomy、claim 边界或跨域验收变化时更新。

## 依赖

- 产品承诺入口：[`玩家入口与发行 PRD`](prd.md)
- Viewer、provider 与 Launcher 合同：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)
- 玩法与 pure API 体验规则：[`doc/game/prd.md`](../../game/prd.md)
- 验证与证据体系：[`doc/testing/prd.md`](../../testing/prd.md)

## 状态

当前任务状态、实现状态和发布结论以绑定 GitHub task issue evidence comments 与上述专业模块 `project.md` 为准；本产品专题不复制这些真值。

## 迁移记录

- 原专题以 `PRD-CORE-009` 建模，已从 `doc/core/` 迁入本产品模块；该 ID 仅用于历史追溯，不把 `core` 重新定义为产品 owner。
- 历史执行项、完成状态和命令留在 `doc/core/project.md` 与对应 GitHub task issue evidence comments；不得把它们复制回产品层。
- 当前产品使用者应先读本专题 PRD，再按其 authority links 下钻到专业域；需要任务真值时，读取绑定 GitHub task issue evidence 与专业模块 `project.md`。

## 维护触发器

- 若新增或撤销玩家访问模式、改变玩家 claim 边界，先更新本专题 PRD，并由 `viewer_engineer`、`agent_engineer`、`qa_engineer` 联审。
- 若变更实现、provider、Launcher、runtime 或测试证据，只更新对应专业域权威；产品层只在承诺或跨域验收改变时回写。
