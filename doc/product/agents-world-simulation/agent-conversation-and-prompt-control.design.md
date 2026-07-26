# Agent 对话与 Prompt 控制产品设计

## 文档身份

- 配对产品 PRD：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md`](agent-conversation-and-prompt-control.prd.md)
- 产品迁移追踪：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.project.md`](agent-conversation-and-prompt-control.project.md)
- 上位产品 PRD：[`doc/product/agents-world-simulation/prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`

本文定义跨 surface 的产品交互设计，不冻结历史 EGUI 布局或实现字段。当前可用入口、鉴权、协议和操作步骤以 [`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md) 与 [`world-simulator` 专业 PRD](../../world-simulator/prd.md) 为准。

## 1. 设计原则

1. 先确认对象，再表达意图：Agent 身份和玩家是否有权操作必须先于输入与应用动作。
2. 对话与配置分层：低风险的一次消息不能与持续影响后续行为的 Prompt/目标调整共用模糊提交语义。
3. 草稿不冒充结果：选择预设、填充、编辑、发送、accepted 与 applied 是不同阶段。
4. 默认值可解释：玩家能看懂当前生效值、默认值、override 与本地草稿之间的关系。
5. 能力按证据收窄：没有 preview、rollback、持久化或完整回执时，不显示或暗示对应保证。

## 2. 信息架构

正式 surface 按以下逻辑层级组织，但不要求固定在某个面板：

1. 当前 Agent：名称/标识、控制资格或 blocked 原因。
2. 交互类型：对话或 Prompt/目标调整。
3. 当前内容来源：默认、当前生效、override、本地草稿或预设。
4. 编辑与主动作：填充、发送、恢复默认、预览、应用或回滚，仅展示当前 authority 支持的动作。
5. 结果与恢复：accepted/applied/rejected/blocked、影响范围、原因和下一步。

预设是草稿辅助层，不与权威 profile 并列成第二事实源。Prompt/目标调整属于高影响层，应比普通对话提供更明确的目标、差异和结果说明。

## 3. 关键交互

### 3.1 预设填充

- 选择预设后先进入可编辑草稿；玩家仍需显式发送。
- 新增、编辑或删除预设只改变当前专业 authority 所声明的预设存储范围。
- 若预设不持久化，应在离开或刷新前说明，而不是让玩家从丢失结果倒推。

### 3.2 加载与编辑 Agent Prompt

- 加载时绑定当前 Agent，并标明值的来源。
- 切换 Agent 时，未提交草稿必须被保留、确认丢弃或明确重载，不能静默应用到另一个 Agent。
- 恢复默认应表达为删除/清除 override 或等价专业语义；清空输入不能在无说明时同时代表“空值”和“恢复默认”。

### 3.3 提交与反馈

- 普通对话使用直接发送反馈；Prompt/目标调整使用更强的目标确认和结果状态。
- 仅当专业 authority 返回实际应用结果时才能显示 applied。只有 acceptance 时，继续显示 pending/accepted 或等价状态。
- rejected/blocked 必须保留原因为玩家可理解的摘要，并提供重新鉴权、刷新当前值、修正输入、选择受支持模式或稍后重试等真实下一步。当前 canonical Web surface 将高级 Prompt 设置默认收起，并在展开后提供 preview/apply/rollback 与最近反馈；这不自动要求其他 surface 具有同等控件。

## 4. 响应式与输入设计

- 内容按“Agent → 类型 → 草稿 → 主动作 → 结果”保持稳定顺序；窄屏可通过分组、渐进披露或内部滚动压缩空间。
- 主动作和结果不得被滚动区永久遮蔽；高影响应用动作与普通发送在位置、文案或确认反馈上应可辨认。
- 输入区必须保持 IME、Enter/换行和焦点行为一致。具体快捷键由当前 surface authority 决定并在手册中说明。
- 诊断细节按需展开，不能盖过目标 Agent、当前值、主要动作和结果。

## 5. 状态模型

| 阶段 | 玩家可见含义 | 不得外推 |
| --- | --- | --- |
| `draft` | 内容只在当前编辑面 | 已发送、已保存 |
| `filled` | 预设已复制到草稿 | 已发送、已应用 |
| `submitted/accepted` | 请求已被入口或 authority 接受 | runtime 已应用、已持久化 |
| `applied` | 当前 authority 确认变更生效 | 永久保存、跨入口完全一致 |
| `rejected/blocked` | 未应用，并给出原因/下一步 | 静默 fallback 或假成功 |

具体状态名和协议枚举可以不同，但玩家语义必须保持等价。

## 6. 设计边界

- 历史“最右 Chat Panel”“Prompt Presets 折叠区”只是设计来源，不是当前布局规范。
- 本设计不决定 profile schema、默认常量、patch 算法、服务端事件来源或强认证方式。
- 本设计不要求恢复已退役 Prompt Ops，也不要求所有 surface 暴露 Agent Prompt 修改能力。
- 技术实现、当前支持范围和验证命令继续由 `world-simulator` 专业文档与证据承载。
