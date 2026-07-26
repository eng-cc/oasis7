# Agent 对话与 Prompt 控制

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[`doc/product/agents-world-simulation/prd.md`](prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.design.md`](agent-conversation-and-prompt-control.design.md)
- 产品迁移追踪：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.project.md`](agent-conversation-and-prompt-control.project.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md)

本文承载玩家与 Agent 对话、复用 Prompt 草稿以及在受支持时调整 Agent Prompt/目标的长期产品承诺。它不声明某个历史 EGUI 面板仍存在，也不拥有协议字段、前端状态、patch 算法、鉴权、持久化或测试步骤。

## 1. 目标

玩家应能在同一条清晰的 Agent 交互路径中区分“发送一次对话”和“调整持续影响 Agent 的 Prompt/目标”，明确当前操作针对哪个 Agent、会产生什么范围的影响，并在提交前后获得与当前 authority 一致的反馈。该入口只面向当前账号已经绑定或按权威规则认领且当前可控制的 Agent；选中、可见或共享世界中的其他 Agent 不因此获得控制权。

预设和草稿用于降低重复输入成本，不是独立决策权威。它们不能绕过身份、Agent 绑定、世界规则、runtime 校验或 provider 边界，也不能把本地填充、请求已发送或表面可达误写为 Agent 配置已经生效。

## 2. 产品承诺

### 2.1 单一、可辨识的交互路径

- 正式 surface 应把 Agent 选择、对话草稿、可用预设与 Prompt 控制组织为一条可理解的路径，避免让玩家在多个含义重叠的入口之间猜测。
- 未绑定、未认领或当前不可控制的 Agent 必须保持只读或明确 blocked；默认选中对象不能绕过账号—Agent authority。
- 对话发送与 Prompt/目标调整必须有明确区分：对话是一条面向 Agent 的消息；配置调整可能影响后续行为，必须使用更强的目标确认、权限和结果反馈。
- 入口布局可以随 Web、native、pure API 或响应式 surface 改变；“右侧面板”“折叠区”或其他历史布局不是产品承诺。

### 2.2 预设与草稿

- 玩家可以在受支持的 surface 中查看、选择、编辑或填充可用预设，再决定是否发送；填充只改变草稿，不等于已经发送、应用或持久化。
- 会话内预设、客户端草稿与权威 Agent profile 是不同状态。surface 必须让玩家能够分辨当前内容来自默认值、当前生效值、本地草稿还是已存在的 override。
- 预设是否跨会话保存、由谁提供以及是否可共享，由当前专业 authority 决定；产品层不承诺本地或服务端持久化。

### 2.3 Agent Prompt 与目标调整

- 当当前模式、身份和专业 authority 支持 Prompt 控制时，玩家在编辑前应看到目标 Agent 与当前生效值，并能编辑系统 Prompt、短期目标或长期目标中被允许的部分。
- 未修改的默认值不应制造无意义的 override。若 surface 把“恢复默认”或清空解释为清除 override，必须在提交前以玩家可理解的方式说明该结果。
- 应用必须返回真实的 accepted、applied、rejected、blocked 或等价结果，并在失败时给出原因与可执行下一步。请求已发送、本地草稿已更新或客户端显示成功都不能代签 runtime 已应用。
- 当前 canonical Web surface 已由专业 authority 提供默认收起的高级 Prompt 设置及 preview/apply/rollback；其他 surface 若不支持其中任一能力、持久化或完整回执，必须明确收窄，不得用“加载当前配置”冒充撤销或恢复保证。

### 2.4 可达性与响应式边界

- 在受支持的窗口和设备尺寸下，Agent 选择、草稿、主要动作、结果与恢复路径必须可达；内容超出可视区域时应提供稳定滚动、分组或渐进披露。
- 响应式收敛不能使输入焦点、输入法、发送动作或高风险配置动作互相混淆；高影响动作不得因空间不足而失去目标和结果说明。
- 对话输入必须保留可理解、可编辑的草稿和明确的发送动作。具体快捷键、组件层级、焦点恢复和布局位置由当前 surface authority 与手册定义；历史 EGUI 的 Enter 发送、最右面板或输入桥接实现不是跨 surface 承诺。

## 3. 范围

覆盖正式玩家 surface 中的 Agent 目标确认、一次对话、预设/草稿、受控 Prompt/目标调整、结果反馈和响应式可达性。不覆盖专业协议、鉴权实现、profile schema、持久化机制、具体 Viewer 组件或测试执行。

## 4. 接口 / 数据

产品层只定义 `目标 Agent → 交互类型 → 内容来源 → 提交阶段 → 权威结果 → 恢复下一步` 的玩家语义。具体 AgentChat、PromptControl、默认值、override、版本、preview/apply/rollback 与反馈字段由 `world-simulator` 专业 authority 定义。

## 5. 里程碑

1. 产品承诺与已退役 EGUI 布局解耦。
2. 产品交互设计集中到同名 design，并由模块 PRD 可达。
3. 当前 Web 权限、操作与自动化合同由 Viewer 手册和专业文档稳定承接。
4. 后续 surface 变更以本专题验收与专业证据共同复核。

## 6. 风险

- 把本地草稿、request acceptance 或 preview 外推为 applied，会制造错误控制承诺。
- 把选中或可见 Agent 当成当前账号可控制 Agent，会绕过认领/绑定 authority。
- 把当前 canonical Web 的控件集合外推给所有 surface，会制造虚假 parity。
- 把已退役 Chat Panel 布局继续当作当前入口，会与 Viewer 手册发生冲突。

## 7. 权威与冲突处理

| 产品层拥有 | 专业域权威 |
| --- | --- |
| 单一交互路径、对话与配置的语义区分、草稿/默认/override 可读性、结果与恢复承诺、响应式可达性 | `doc/world-simulator/prd.md` 拥有 AgentChat、PromptControl、模式、鉴权与 Viewer 合同；Viewer 手册拥有当前 surface 和操作边界；runtime/provider 专业权威拥有实际应用、拒绝和世界影响 |

历史 Standard-3D / EGUI Chat Panel、Prompt Ops、字段名、状态结构和 patch 构造只能作为迁移来源或 Git 历史追溯，不能覆盖当前 Viewer 手册与专业 authority。

## 8. 组合验收

- AC-1：玩家能在受支持 surface 中辨认当前目标 Agent，并区分一次对话、草稿填充与持续 Prompt/目标调整。
- AC-2：预设填充不会被呈现为已发送或已应用；本地草稿、默认值、当前生效值和 override 不会被混成同一状态。
- AC-3：适用的 Prompt 控制明确呈现影响范围、目标 Agent、提交结果和失败恢复；请求 acceptance 不会被外推为 applied。
- AC-4：恢复默认或清除 override 的语义在提交前可理解，未修改默认值不会制造无意义变更。
- AC-5：不支持 preview、rollback、持久化或完整回执的 surface 明确收窄，不暗示完整恢复闭环。
- AC-6：窄屏或低高度下仍可到达草稿、主要动作、结果与恢复路径，且输入法、发送与高影响配置动作不会互相误触。
- AC-7：对话草稿支持正常文本编辑；surface 未提供或未验证快捷键、IME 组合态或自动聚焦保证时，不得把历史实现描述成当前能力。

## 9. 验收追踪

| 成功标准 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| AC-1 / AC-2 | producer_system_designer / viewer_engineer | `doc/world-simulator/prd.md`; Viewer 手册 | 当前正式 surface 的 Agent 选择、对话、草稿与状态来源对账 | test_tier_required |
| AC-3 / AC-4 / AC-5 | agent_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; 对应 PromptControl/runtime authority | accepted/applied/rejected/blocked、默认/override 与缺失能力负例 | test_tier_required |
| AC-6 | game_visual_interaction_designer / viewer_engineer / qa_engineer | Viewer 视觉规范、手册与当前实现 authority | desktop、窄屏、低高度及输入法/焦点交互证据 | test_tier_required |

## 10. Non-Goals

- 不规定右侧面板、折叠区、按钮文案、组件层级或具体布局常量。
- 不定义 `AgentChat`、`PromptControl`、profile 字段、patch、鉴权或持久化实现。
- 不承诺当前所有入口都支持预设编辑、Prompt apply、preview、rollback 或跨会话保存。
- 不保存历史任务状态、截图 verdict、测试命令或已退役 surface 的能力说明。
