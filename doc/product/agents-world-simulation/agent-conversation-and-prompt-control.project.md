# Agent 对话与 Prompt 控制迁移追踪

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md`](agent-conversation-and-prompt-control.prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.design.md`](agent-conversation-and-prompt-control.design.md)
- 上位产品 PRD：[`doc/product/agents-world-simulation/prd.md`](prd.md)
- 追踪范围：产品文档语义迁移
- Owner role：`repository_health_engineer`

本文只记录产品层迁移映射与删除条件，不复制旧实现任务、测试步骤、完成状态或 GitHub task evidence。

## 迁移映射

| 已吸收源专题 | 归位语义 | 后续权威 |
| --- | --- | --- |
| `viewer-chat-prompt-presets` 三件套 | 单一交互路径、预设编辑与填充、草稿不等于发送/持久化 | 产品 PRD/design；当前 surface 与 AgentChat 合同留在 `world-simulator` |
| `viewer-chat-prompt-presets-profile-editing` 三件套 | 目标 Agent、当前值加载、Prompt/目标编辑、apply 与缺失 rollback 边界 | 产品 PRD/design；PromptControl、鉴权和应用结果留在 `world-simulator` |
| `viewer-chat-prompt-presets-scroll` 三件套 | 窄屏和低高度下的可达性、滚动与焦点边界 | 产品 design；布局实现与验证留在 Viewer 专业权威 |
| `viewer-chat-agent-prompt-default-values-prefill` 三件套 | 默认值、当前生效值、草稿与 override 的玩家语义 | 产品 PRD/design；字段和 patch 算法留在 `world-simulator` |

## 删除收据

- 源文件数量：12。
- 迁移结果：产品承诺进入同名 PRD，产品交互设计进入同名 design，迁移映射进入本 project。
- 专业语义保留：当前 surface 由 Viewer 手册描述；AgentChat、PromptControl、鉴权、profile、patch、runtime 应用与验证仍归 `doc/world-simulator` 及对应代码/测试 authority。
- 历史追溯：源三件套删除后只通过 Git history 与 GitHub task issue evidence 进入，不再保留日期化或实现表面绑定的重复入口。

## 完成条件

本迁移收据只在以下产品文档事实同时成立时有效：产品模块主 PRD 可达本专题且专题回链主 PRD；产品承诺、产品设计与专业实现边界已分离；已退役 EGUI 布局未被提升为当前产品能力；活跃索引和 Viewer landing 已改指当前权威；12 个源文件已整组删除。合并、review 和任务完成事实不由本文件证明。

## 任务拆解

不适用。本文件不维护执行计划、checkbox、owner 队列或当前任务进度；相关真值只进入 GitHub task issue evidence。

## 依赖

- [`doc/product/README.md`](../README.md) 的迁移治理与产品/专业边界。
- [`doc/world-simulator/prd.md`](../../world-simulator/prd.md) 的 AgentChat、PromptControl 与 runtime authority。
- [`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md) 的当前 surface、权限与操作合同。

## 状态

- 文档生命周期：`active`。
- 迁移收据：`finalized`，表示上方 12 个源文件的产品语义和专业边界已归位；不表示 PR、review、CI、发布或任务完成。
- 历史追溯：旧实现任务与测试证据只从 Git history / GitHub task issue evidence 进入。
