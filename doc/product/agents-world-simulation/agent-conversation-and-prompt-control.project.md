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

- [x] 产品模块主 PRD 可达本专题，且专题回链主 PRD。
- [x] 产品承诺、产品设计与专业实现边界分离。
- [x] 已退役 EGUI 布局未被提升为当前产品能力。
- [x] `doc/world-simulator/prd.index.md` 与 Viewer landing 的活跃引用改指当前权威。
- [x] 12 个源文件整组删除。
- [ ] 合并前治理检查、精确旧路径断言与 involved-role review 通过。

## 任务拆解

- [x] 盘点 12 个源文件的产品、设计、专业实现与历史状态语义。
- [x] 建立长期产品 PRD/design/project，并补强当前 Viewer 专业入口。
- [x] 修复活跃索引和 landing 引用，删除全部已吸收源文件。
- [ ] 完成治理验证、专业 review 与合并收口。

## 依赖

- [`doc/product/README.md`](../README.md) 的迁移治理与产品/专业边界。
- [`doc/world-simulator/prd.md`](../../world-simulator/prd.md) 的 AgentChat、PromptControl 与 runtime authority。
- [`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md) 的当前 surface、权限与操作合同。
- GitHub issue #2597 的任务、review 与合并证据。

## 状态

- 当前阶段：语义迁移和源文件删除已完成，等待治理验证与 involved-role review。
- 残余迁移债务：无；旧实现任务与测试证据只从 Git history / GitHub task issue evidence 追溯。
