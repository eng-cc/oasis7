# Agent 对话与 Prompt 控制

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[`doc/product/agents-world-simulation/prd.md`](prd.md)
- 配对产品设计：[`doc/product/agents-world-simulation/agent-conversation-and-prompt-control.design.md`](agent-conversation-and-prompt-control.design.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md)

本文承载玩家与 Agent 对话、复用 Prompt 草稿以及在受支持时调整 Agent Prompt/目标的长期产品承诺。它不声明某个历史 EGUI 面板仍存在，也不拥有协议字段、前端状态、patch 算法、鉴权、持久化或测试步骤。

## 1. 目标

### 1.1 代表性情境

玩家正在与一个自己有权控制的 Agent 协作：他想先发送一条消息确认当前状态，再把一项持续目标调整为更合适的方向。玩家需要在提交前看懂对象、权限、草稿来源和影响范围，在提交后分辨请求只是被接受、已经应用、被拒绝还是因控制权变化而停止；如果另一入口先改了内容或玩家失去资格，他需要保留安全的本地草稿并知道下一步，而不是猜测旧内容是否已经生效。

### 1.2 事实、假设与未决问题

- 已验证产品事实：本专题已有稳定 `REQ-PROMPT-*` / `AC-PROMPT-*` 追踪，且产品语义与配对 design、`world-simulator` authority 分开维护。
- 设计假设：在受支持的入口中，明确区分对象、权限、草稿、接受和应用会帮助玩家作出可归因的下一次决策；该假设不能由结构检查或一次提交成功代替。
- 产品级未决问题：无未决的产品行为义务；各入口是否支持预设、preview、rollback、持久化和完整回执，仍须由当前 Viewer 手册、`world-simulator`/runtime authority 和适用证据在声称该能力前决定。接收 owner 为对应专业 role，触发条件是该入口进入本专题承诺范围；当前任务证据保留所用范围与 residual risk。

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

### 2.5 并发变更与过期草稿

草稿加载时只建立本次编辑的比较基准：目标 Agent、当时的当前生效值以及玩家仍可证明的控制/授权范围。这个基准不是权威写入凭证，也不能替代提交时的重新校验。

- 判定顺序必须先校验目标 Agent 与当前控制/授权；控制或授权丢失优先于单纯的生效值变化。只有控制/授权仍有效时，才允许把“当前值已变化”解释为可查看的 stale/冲突。
- 若编辑期间目标 Agent 的绑定、转让、撤销或玩家控制资格发生变化，原草稿必须保持本地未提交并进入 `blocked/control-lost` 或等价语义；surface 只能显示不泄露权威内容的安全原因（例如当前已不具备控制资格），不得显示最新权威值或差异，也不得提供该草稿的提交、重放或转移路径。玩家可以放弃本地草稿或进入另一个明确授权的恢复/重新选择流程，但不能借旧草稿恢复控制。
- 若控制/授权仍有效，而当前生效的 Prompt/目标已被另一 surface、owner/组织或其他权威变更，提交必须识别为过期或冲突。surface 应保留玩家草稿、显示最新生效值与变化原因，并提供明确的查看差异、刷新/重载、在当前值上重新编辑、放弃或其他专业 authority 支持的下一步；不得自动合并、覆盖最新值、把旧草稿重放，或把本地草稿表示为 applied。
- 只有目标、控制资格、当前生效值和适用授权在提交时仍通过重新校验，草稿才可作为新的独立请求提交。提交期间若再次发生竞态，玩家只能看到 authority 返回的真实 accepted、applied、rejected、blocked、stale 或等价结果；不得用乐观成功、静默重试或重复提交掩盖冲突。

该边界不冻结 revision 字段、合并算法或冲突协议；它只确保控制/授权丢失时不泄露或重放权威内容，并确保在授权仍有效时，外部值变化不会被旧草稿无声覆盖。

### 2.6 稳定叶子要求

以下叶子要求把本专题原有承诺拆成可单独引用的产品合同；它们不新增协议字段、组件或实现状态。每条的设计细节继续由配对 design 与专业 authority 承接。

<a id="req-prompt-001"></a>
#### REQ-PROMPT-001：目标 Agent 与控制资格

- 适用条件：玩家进入对话或 Prompt/目标调整入口。
- 要求：入口必须明确当前目标 Agent 和玩家当前控制/授权结果；选中、可见或共享世界中的 Agent 不得被呈现为因此可控制。
- 上位承诺：2.1 单一、可辨识的交互路径。
- 专业权威：[`world-simulator` Agent/鉴权合同](../../world-simulator/prd.md)。
- 验收：AC-PROMPT-001。

<a id="req-prompt-002"></a>
#### REQ-PROMPT-002：对话、草稿与配置区分

- 适用条件：玩家查看、填充、编辑或发送内容。
- 要求：入口必须区分一次对话、预设填充、本地草稿和持续 Prompt/目标调整；填充或编辑不得被呈现为已发送或已应用。
- 上位承诺：2.1、2.2。
- 专业权威：[配对产品 design 的信息架构与状态模型](agent-conversation-and-prompt-control.design.md#2-信息架构)。
- 验收：AC-PROMPT-002。

<a id="req-prompt-003"></a>
#### REQ-PROMPT-003：默认值与 override 语义

- 适用条件：当前内容来自默认值、当前生效值、override、草稿或预设。
- 要求：入口必须让玩家分辨这些来源；恢复默认或清除 override 必须在提交前表达真实含义，清空输入不得无说明地同时表示空值和恢复默认。
- 上位承诺：2.2、2.3。
- 专业权威：[`world-simulator` 专业 PRD](../../world-simulator/prd.md) 与 [`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md)。
- 验收：AC-PROMPT-003。

<a id="req-prompt-004"></a>
#### REQ-PROMPT-004：接受不等于应用

- 适用条件：入口或 authority 返回请求接受，但没有实际应用结果。
- 要求：入口必须保持 `accepted/pending` 或等价语义；只有 authority 确认实际应用时才可以显示 `applied`。
- 上位承诺：2.3、4 接口/数据。
- 专业权威：[`world-simulator` 专业 PRD](../../world-simulator/prd.md) 与 [`world-runtime` 专业 PRD](../../world-runtime/prd.md)。
- 验收：AC-PROMPT-004。

<a id="req-prompt-005"></a>
#### REQ-PROMPT-005：按证据收窄能力

- 适用条件：某 surface 不支持 preview、rollback、持久化或完整回执。
- 要求：该 surface 必须明确收窄能力，不得以加载配置、请求已发送或表面成功暗示未被 authority 支持的恢复或持久性保证。
- 上位承诺：2.3、3 范围。
- 专业权威：[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md) 与 [`world-simulator` 专业 PRD](../../world-simulator/prd.md)。
- 验收：AC-PROMPT-005。

<a id="req-prompt-006"></a>
#### REQ-PROMPT-006：输入与响应式可达性

- 适用条件：受支持的窗口、设备尺寸和文本输入场景。
- 要求：目标 Agent、草稿、主动作、结果和恢复路径必须可达；输入法、焦点、换行、发送和高影响配置动作不得互相误触。未验证的历史快捷键或布局不得被写成当前保证。
- 上位承诺：2.4。
- 专业权威：[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md) 与 [`world-simulator` 专业 PRD](../../world-simulator/prd.md)。
- 验收：AC-PROMPT-006。

<a id="req-prompt-007"></a>
#### REQ-PROMPT-007：控制权丢失优先于值变化

- 适用条件：编辑期间 Agent 绑定、转让、撤销、owner/组织授权或玩家控制资格发生变化。
- 要求：入口必须先按 `blocked/control-lost` 或等价语义处理；草稿只能保持未提交并显示 authority-safe 原因，不得展示最新权威值/差异，也不得提供提交、重放或转移旧草稿的路径。
- 上位承诺：2.5 并发变更与过期草稿。
- 专业权威：[`world-simulator` Agent/鉴权合同](../../world-simulator/prd.md) 与 [`world-runtime` 专业 PRD](../../world-runtime/prd.md)。
- 验收：AC-PROMPT-007。

<a id="req-prompt-008"></a>
#### REQ-PROMPT-008：授权仍有效时的 stale 恢复

- 适用条件：控制/授权仍有效，但当前生效 Prompt/目标被其他合法 authority 改变。
- 要求：入口必须保留本地草稿并明确 stale/needs-refresh，展示允许披露的最新值和变化原因，提供受支持的差异、刷新/重载、重新编辑或放弃路径；不得静默合并、覆盖、自动重试或把草稿表示为 applied。
- 上位承诺：2.5。
- 专业权威：[`world-simulator` 专业 PRD](../../world-simulator/prd.md)、[`world-runtime` 专业 PRD](../../world-runtime/prd.md)。
- 验收：AC-PROMPT-008。

<a id="req-prompt-009"></a>
#### REQ-PROMPT-009：提交时重新校验与单一结果

- 适用条件：玩家提交编辑草稿，或提交期间再次发生竞态。
- 要求：提交必须重新校验目标、控制资格、当前生效值和适用授权；竞态只能呈现 authority 的一个真实 accepted/applied/rejected/blocked/stale 结果，不得乐观显示成功、静默重试或重复提交。
- 上位承诺：2.5。
- 专业权威：[`world-runtime` 专业 PRD](../../world-runtime/prd.md) 与 [`testing` 专业 PRD](../../testing/prd.md)。
- 验收：AC-PROMPT-009。

<a id="req-prompt-010"></a>
#### REQ-PROMPT-010：真实失败下一步

- 适用条件：出现非 `control-lost` 的 rejected/blocked 或受支持恢复失败。
- 要求：入口必须保留玩家可理解的原因摘要和真实下一步，例如重新鉴权、刷新、修正输入、选择受支持模式或稍后重试；不得用静默 fallback、假成功或无边界等待代替恢复。
- 上位承诺：2.3、2.5。
- 专业权威：[`Viewer 手册`](../../world-simulator/viewer/viewer-manual.manual.md)、[`world-runtime` 专业 PRD](../../world-runtime/prd.md) 与 [`world-simulator` 专业 PRD](../../world-simulator/prd.md)。
- 验收：AC-PROMPT-010。

<a id="req-prompt-011"></a>
#### REQ-PROMPT-011：applied 的范围边界

- 适用条件：专业 authority 已确认本次 Prompt/目标变更实际应用，但没有另行确认永久保存或其他入口同步。
- 要求：入口可以显示 `applied`，但必须把它限定为当前 authority 声明的本次应用范围；除非有对应 authority 证据，不得把 applied 外推为永久保存、跨会话持久化或跨入口同步。
- 上位承诺：2.3、3 范围。
- 专业权威：[`world-simulator` 专业 PRD](../../world-simulator/prd.md)、[`world-runtime` 专业 PRD](../../world-runtime/prd.md)。
- 验收：AC-PROMPT-011。

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
- AC-3：适用的 Prompt 控制明确呈现影响范围、目标 Agent、提交结果和失败恢复；请求 acceptance 不会被外推为 applied，单次 applied 也不会被外推为永久保存或跨入口同步。
- AC-4：恢复默认或清除 override 的语义在提交前可理解，未修改默认值不会制造无意义变更。
- AC-5：不支持 preview、rollback、持久化或完整回执的 surface 明确收窄，不暗示完整恢复闭环。
- AC-6：窄屏或低高度下仍可到达草稿、主要动作、结果与恢复路径，且输入法、发送与高影响配置动作不会互相误触。
- AC-7：对话草稿支持正常文本编辑；surface 未提供或未验证快捷键、IME 组合态或自动聚焦保证时，不得把历史实现描述成当前能力。
- AC-8：代表性并发/外部变更样例证明判定顺序为“先控制/授权、后值变化”：控制资格、owner/组织授权或目标绑定丢失时，旧草稿保留为未提交的 `blocked/control-lost`，只显示 authority-safe 原因，不显示最新权威值/差异，也不提供该草稿的提交、重放或转移路径；仅当控制/授权仍有效且当前生效 Prompt/目标变化时，才进入 `stale` 并显示最新值/差异及显式刷新、重载、重新编辑或放弃路径。两类结果都不会自动合并、覆盖、转移、重放或表达为 applied，竞态提交只产生一个真实 authority 结果。

### 8.1 叶子场景

<a id="ac-prompt-001"></a>
#### AC-PROMPT-001：当前对象可控制性

给定：玩家进入一个支持对话或 Prompt 控制的入口。
当：入口加载或玩家切换 Agent。
则：入口明确展示当前目标 Agent 与控制/授权结果；不可控制对象保持只读或 blocked，不因选中或可见获得控制承诺。

覆盖：REQ-PROMPT-001。

<a id="ac-prompt-002"></a>
#### AC-PROMPT-002：草稿不冒充提交

给定：玩家选择预设并编辑 Prompt/目标。
当：内容仍在本地草稿阶段。
则：入口区分预设、草稿、当前生效值和一次对话；不会显示已发送、已应用或已持久化。

覆盖：REQ-PROMPT-002。

<a id="ac-prompt-003"></a>
#### AC-PROMPT-003：默认与 override 可解释

给定：当前内容来自默认值或已有 override。
当：玩家选择恢复默认、清除 override 或清空输入。
则：提交前能理解实际语义，未修改默认值不产生无意义变更。

覆盖：REQ-PROMPT-003。

<a id="ac-prompt-004"></a>
#### AC-PROMPT-004：accepted 不扩大为 applied

给定：入口返回接受确认，但没有实际应用回执。
当：入口呈现请求结果。
则：显示 accepted/pending 或等价状态，不显示 applied、永久保存或跨入口同步。

覆盖：REQ-PROMPT-004。

<a id="ac-prompt-005"></a>
#### AC-PROMPT-005：能力按 evidence 收窄

给定：当前 surface 不支持 preview、rollback、持久化或完整回执中的一项。
当：玩家查看可用动作和结果。
则：入口明确未支持的边界，不以加载当前值或请求成功暗示完整恢复闭环。

覆盖：REQ-PROMPT-005。

<a id="ac-prompt-006"></a>
#### AC-PROMPT-006：窄屏与文本输入可达

给定：在受支持的窄屏或低高度窗口中编辑文本。
当：玩家输入、换行、发送或查看结果。
则：目标、草稿、主动作、结果和恢复路径仍可达；未验证的历史快捷键、IME 或布局不会被宣称为保证。

覆盖：REQ-PROMPT-006。

<a id="ac-prompt-007"></a>
#### AC-PROMPT-007：控制权丢失优先

给定：玩家有未提交草稿，编辑期间控制资格丢失且当前值也发生变化。
当：入口重新校验对象和权限。
则：按 blocked/control-lost 处理，只显示安全原因；不披露最新值/差异，不提供旧草稿提交、重放或转移。

覆盖：REQ-PROMPT-007。

<a id="ac-prompt-008"></a>
#### AC-PROMPT-008：合法外部变化可恢复

给定：玩家仍有控制资格，另一合法入口改变了当前生效值。
当：玩家尝试提交原草稿。
则：保留本地草稿并呈现 stale；不静默覆盖或合并，并提供受支持的比较、刷新/重载、重新编辑或放弃路径。

覆盖：REQ-PROMPT-008。

<a id="ac-prompt-009"></a>
#### AC-PROMPT-009：竞态只产生一个真实结果

给定：玩家提交一个编辑草稿；可以是目标、权限和当前值在提交前后均未变化的普通提交，也可以是提交期间发生目标、权限或当前值竞态的提交。
当：入口提交并收到 authority 结果。
则：两种分支都必须按当前状态重新校验目标、控制资格、当前生效值和适用授权；无竞态分支在校验通过后呈现一个 authority 真实结果，竞态分支也只呈现一个 authority 真实结果；两者都不会先显示 applied 再回滚、静默重试或复制提交。

覆盖：REQ-PROMPT-009。

<a id="ac-prompt-010"></a>
#### AC-PROMPT-010：失败有真实下一步

给定：请求被非 control-lost 的 rejected/blocked，或玩家执行一个 authority 支持的恢复动作后该恢复动作失败。
当：入口呈现原始请求或恢复动作的失败结果。
则：展示可理解原因和至少一个 authority 支持的下一步；无法安全恢复时明确停止和边界，不以无限等待或静默 fallback 掩盖恢复失败。

覆盖：REQ-PROMPT-010。

<a id="ac-prompt-011"></a>
#### AC-PROMPT-011：applied 不扩大为持久化或同步

给定：authority 已确认本次变更实际应用，但没有确认永久保存、跨会话持久化或其他入口同步。
当：入口呈现 applied 结果并说明影响范围。
则：入口可以显示 applied，但同时保留当前 authority 声明的范围；不得显示永久保存、跨会话持久化或跨入口同步已经成立。只有取得相应 authority 证据，才可以增加这些后续语义。

覆盖：REQ-PROMPT-011。

## 9. 验收追踪

| 成功标准 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| AC-1 / AC-2 | producer_system_designer / viewer_engineer | `doc/world-simulator/prd.md`; Viewer 手册 | 当前正式 surface 的 Agent 选择、对话、草稿与状态来源对账 | test_tier_required |
| AC-3 / AC-4 / AC-5 | agent_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; 对应 PromptControl/runtime authority | accepted/applied/rejected/blocked、applied 范围、持久化/同步外推负例、默认/override 与缺失能力负例 | test_tier_required |
| AC-6 | game_visual_interaction_designer / viewer_engineer / qa_engineer | Viewer 视觉规范、手册与当前实现 authority | desktop、窄屏、低高度及输入法/焦点交互证据 | test_tier_required |
| AC-8 | producer_system_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 外部值/授权变化、目标转让或撤销、过期草稿的 stale/blocked 可读性、显式刷新/重编辑与竞态提交至多一个真实 authority 结果的组合证据 | test_tier_full |

### 9.1 叶子追踪

| 叶子要求 | 原有承诺/来源 | 验收场景 | 配对 design 锚点 |
| --- | --- | --- | --- |
| REQ-PROMPT-001 | 2.1；1 设计原则 | AC-PROMPT-001 | design §1、§2 |
| REQ-PROMPT-002 | 2.1–2.2；AC-1/AC-2 | AC-PROMPT-002 | design §1、§2、§3.1 |
| REQ-PROMPT-003 | 2.2–2.3；AC-2/AC-4 | AC-PROMPT-003 | design §3.2、§6 |
| REQ-PROMPT-004 | 2.3；AC-3 | AC-PROMPT-004 | design §3.4、§5 |
| REQ-PROMPT-005 | 2.3、3；AC-5 | AC-PROMPT-005 | design §1、§6 |
| REQ-PROMPT-006 | 2.4；AC-6/AC-7 | AC-PROMPT-006 | design §4 |
| REQ-PROMPT-007 | 2.5；AC-8 | AC-PROMPT-007 | design §3.3、§5 |
| REQ-PROMPT-008 | 2.5；AC-8 | AC-PROMPT-008 | design §3.3、§5 |
| REQ-PROMPT-009 | 2.5、4；AC-8 | AC-PROMPT-009 | design §3.3、§6 |
| REQ-PROMPT-010 | 2.3、6；AC-3/AC-8 | AC-PROMPT-010 | design §3.4、§5 |
| REQ-PROMPT-011 | 2.3、3；AC-3 | AC-PROMPT-011 | design §3.4、§5 |

## 10. Non-Goals

- 不规定右侧面板、折叠区、按钮文案、组件层级或具体布局常量。
- 不定义 `AgentChat`、`PromptControl`、profile 字段、patch、鉴权或持久化实现。
- 不承诺当前所有入口都支持预设编辑、Prompt apply、preview、rollback 或跨会话保存。
- 不保存历史任务状态、截图 verdict、测试命令或已退役 surface 的能力说明；这类可变追溯只进入 GitHub task issue evidence。
