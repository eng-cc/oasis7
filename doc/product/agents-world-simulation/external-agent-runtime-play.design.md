# 外部 Agent Runtime 接入与持续游玩产品设计

## 文档身份

- 配对产品 PRD：[external-agent-runtime-play.prd.md](external-agent-runtime-play.prd.md)
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[Decision Provider](../../world-simulator/llm/decision-provider-contract.prd.md)、[Continuous Agent Harness](../../world-simulator/llm/continuous-agent-harness.prd.md)、[双轨执行](../../world-simulator/llm/provider-agent-dual-mode.prd.md)、[Runtime lifecycle](../../world-runtime/runtime/agent-cognition-lifecycle.prd.md)
- Last reviewed：2026-10-10

本设计组织配对 PRD 的用户经历、信息层级和恢复解释。所有行为义务和首期范围由配对 PRD 拥有；以下状态是体验表达，不新增协议枚举、CLI 参数、组件布局或实现状态机，也不表示当前已支持 OpenClaw/Codex。

## 1. 设计命题

目标体验是让玩家把已经使用的 Runtime 委托给自己的游戏 Agent，能够放心离开观战界面，并在回来后理解进展、损失、阻塞和下一步。玩家主要决定目标、策略约束、委托范围与是否继续投入；Runtime 自主决定授权内的具体行动。

接入阶段优先回答“连接的是谁、替哪个 Agent 做什么、需要什么条件和开销”；开始后优先回答“目标有何进展、世界确认了什么、下一步为何这样做”。版本、适配器、连接诊断和详细调用记录按需展开，不能挤占游戏目标与后果。

设计假设是：熟悉的 Runtime 使用方式、连续的目标反馈和明确的恢复选项能降低接入与持续游玩的理解成本。反证信号包括：用户必须读原始日志才知道结果、重复输入逐步命令、把对话回复当成已应用目标，或因一次断连重新创建任务和重复操作。真实使用者观察验证这些假设；结构检查不证明吸引力或留存。

## 2. 代表性使用经历

### 2.1 首次接入与委托

玩家选择 OpenClaw 或 Codex 的推荐接入说明。入口明确区分“目标 Runtime”“已验证组合”“不适用版本或缺失能力”，并给出连接目标、凭据归属、运行前置和成本范围。仅检测到程序存在或模型能回答时，仍保留尚未完成的连接/会话步骤。

连接就绪后展示当前身份、合法可控制 Agent 和其既有义务。玩家确认生产目标及有效委托；如果没有可控制 Agent，转到既有认领或观察路径。认领与维护条件消费[所有权主责](../world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#1-产品承诺)，连接软件本身不授予实体或资源。

启动前给出一个完整的委托摘要：目标世界与 Agent、高层目标、允许范围、预算的可执行部分、持续运行条件。日常行动由 Runtime 在该范围内自行选择；严重后果和额外授权消费[自治与委托主责](agent-authority-ownership-and-accountability.prd.md#2-授权与自治边界)，避免把正常游玩变成连续的逐动作批准。

### 2.2 生产、反馈与新的选择

代表性任务消费[当前首局链](../world-rules-core-gameplay/first-session-and-continuation.prd.md)：Runtime 从观察中识别目标、资源和合法能力，形成计划并推进生产。计划可以变化，也可能因资源、授权或观察过期而被拒绝；产品将变化解释为“发生了什么、为什么需要调整、有哪些有效下一步”。

世界结算后，将匹配的成果、资源变化和主因果带回任务上下文。下一轮 Runtime 以真实结果继续，玩家同时能看到对应成果。生产接受、排程、普通库存变化与首产物完成按首局主责区分；这里不增加新的完成口径。

第一次遇到生产阻塞时，玩家可以理解等待、补足条件、缩小目标或重排之间的取舍。界面只展示当时权威合同支持的选择；无法执行的选项说明限制，不用反复重试代替决策。任务取得首产物后继续展示下一项目标与未完成义务，不自动宣布稳定生产或交付完成。

### 2.3 目标调整、离开与回访

玩家可以保持委托、调整高层目标或停止后续委托。目标调整沿用[Prompt 主责](agent-conversation-and-prompt-control.prd.md#23-agent-prompt-与目标调整)：草稿、已接受和已应用分别呈现。只有应用后的新决策使用有效新目标；已有待决行动独立显示，不能借调整目标暗示其已经取消。

离开 Viewer 时，执行方仍在线且委托有效，任务可以继续。回访先恢复当前目标、最新可信成果、主要阻塞和安全下一步，再提供离开期间的详细记录。关闭 Runtime 时如实说明其执行停止；世界继续推进，已受理行动仍按原合同产生结果。

## 3. 状态与恢复

### 3.1 分开表达四类状态

| 状态面 | 玩家需要区分 | 表达原则 |
| --- | --- | --- |
| 观战/玩家会话 | 在线、断连、会话待恢复 | 仅说明观察或提交入口是否可用；不代签 Runtime 停止或 Agent 委托失效。 |
| Runtime 执行 | 未就绪、运行、等待、阻塞、离线、恢复中 | 说明是否能产生下一次决策及主要原因；等待须有可理解的条件或下一步。 |
| Agent 委托与目标 | 当前有效、目标待应用、到期/撤销、需重新确认 | 有效资格优先于缓存目标；恢复连接不自动恢复权限。 |
| 世界行动与成果 | 候选、待决、已结算、拒绝/失效、结果待核对 | 消费权威结果；执行连接状态、对话回复和本地日志不能改写此面。 |

多种状态并存时，主界面先显示影响下一步的资格/安全阻塞，再显示待核对世界结果、可执行恢复、当前目标进展；已结算成果始终保留。网络故障不能把成果归零，也不能将离线期间的推测填成最新事实。

### 3.2 主要故障的处理经历

| 触发 | 保留什么 | 用户下一步 | 恢复后的判据 |
| --- | --- | --- | --- |
| Runtime 版本或能力不符 | 已确认身份、目标草稿与尚未开始的任务 | 按推荐说明修复适配或选择适用目标 | 真正由声明 Runtime 承担会话，所需能力可用。 |
| 连接认证或委托失效 | 权威成果、原请求关联和失效原因 | 重新认证、取得合法委托，或保持观察 | 重新核对资格；不得以公开标识、旧缓存或费用路由替代授权。 |
| 世界拒绝或资源不足 | 已发生消耗、实际成果及当前主要阻塞 | 刷新观察、补足、等待或重排 | 新决策依据新事实；拒绝不会变成模型自报成功。 |
| 超时、响应丢失或状态未知 | 原请求身份与已知世界进度 | 核对原结果、等待或走受支持恢复 | 确认后继续，不隐式重做未知动作或静默启动另一个 Runtime。 |
| Runtime 进程重启 | 可恢复会话信息与可信世界历史 | 核对权限、目标和未决义务后续接；私有上下文缺失时重新规划 | 旧成果不重复，新决策来自仍有效的目标与权限。 |
| 预算耗尽或计量不完整 | 成果、待决结果、已知开销与未知部分 | 停止新受限请求，补充预算或结束委托 | 可执行限制与估计明确区分，不能承诺无法落实的下游硬上限。 |

恢复是核对已有工作再决定下一步。用户若显式选择切换 Runtime，消费[provider 切换主责](provider-agent-experience-continuity.prd.md#21-切换窗口与在途意图)；首期不要求把原 Runtime 的私有记忆、工具进程和在途推理无损迁移到另一个 Runtime。

## 4. 信息层级与入口一致性

### 4.1 默认任务视图

默认层保留目标世界与 Agent、当前有效目标、Runtime 是否在工作、最近权威成果、主要阻塞、预算摘要和下一步。只有与当前决策相关的连接或权限问题前景化。

展开层提供阶段计划摘要、行动与结果关联、等待原因、目标变更和恢复历史。诊断层提供版本、适配方式、模型/profile、调用开销、错误与证据引用；可以核对实际 Runtime 和会话来源，无需暴露凭据、其他 Agent 私有上下文或模型内部思维过程。

工具调用记录能说明 Runtime 尝试了什么；世界 receipt 说明实际发生了什么。两者通过同一任务关系可追溯，展示上保持区别。

### 4.2 Viewer 与 pure API

`viewer` 提供玩家可读的上述信息和间接控制；`pure_api` 提供语义等价、可由客户端消费的状态、目标调整与恢复结果。两者复用世界事实，并分别证明自身入口可用性；产品不要求 pure API 自带图形界面。

执行 lane 的可见范围不随玩家界面故障自动扩大。无 GUI 运行也可以保持受约束的观察；需要改变 lane 时按专业合同显式选择并记录，已有证据不能改标成另一入口或观察范围的成功。

窄屏、语言变化和回访的信息保留消费[玩家可读表面连续性](player-readable-surface-continuity.prd.md)；布局调整不能藏起当前 Agent、资格阻塞、待核对结果或主要恢复动作。

## 5. 资源、取舍与验证

世界维护和工业消耗遵循玩法与 Runtime 主责；外部模型/工具成本单独说明。开始前让用户理解“已知会消耗什么、哪些仅是估计、哪层能阻止新增开销”；执行中让用户根据实际进展决定继续、缩小目标、调整预算或停止后续委托。停止不回滚世界，也不保证已发出的推理立即停止计费。

首期优先一条完整首局和两个真实 Runtime 的独立适配。更多工具、长期记忆迁移和多 Agent 协作可以扩展，但不能用扩大功能目录替代现有路径的反馈、目标生效和恢复完整性。

产品评审观察用户能否独立接入、辨认所用 Runtime、理解有效目标与权威结果、处理一次阻塞并回访继续。专业验证核对实际 Runtime 会话、能力执行、反馈、权限、预算和恢复。未经改变的 builtin 基线、额外脚本辅助和 Runtime 的局部成功应按专业 parity 如实记录；该设计不规定统一模型策略或新的分数阈值。

## PRD REQ/AC fragment mapping

| 产品要求 | 产品验收 | 本文设计承接 |
| --- | --- | --- |
| [REQ-EXT-001](external-agent-runtime-play.prd.md#req-ext-001) | [AC-EXT-001](external-agent-runtime-play.prd.md#ac-ext-001) | §2.1 推荐路径、真实 Runtime 与就绪表达。 |
| [REQ-EXT-002](external-agent-runtime-play.prd.md#req-ext-002) | [AC-EXT-002](external-agent-runtime-play.prd.md#ac-ext-002) | §2.1 委托摘要；§3 资格与恢复。 |
| [REQ-EXT-003](external-agent-runtime-play.prd.md#req-ext-003) | [AC-EXT-003](external-agent-runtime-play.prd.md#ac-ext-003) | §2.2 合法能力、前置变化与首局结果。 |
| [REQ-EXT-004](external-agent-runtime-play.prd.md#req-ext-004) | [AC-EXT-004](external-agent-runtime-play.prd.md#ac-ext-004) | §2.1–2.2 自主推进；§4 计划与反馈层次。 |
| [REQ-EXT-005](external-agent-runtime-play.prd.md#req-ext-005) | [AC-EXT-005](external-agent-runtime-play.prd.md#ac-ext-005) | §2.2 权威后果；§3 未知结果与核对。 |
| [REQ-EXT-006](external-agent-runtime-play.prd.md#req-ext-006) | [AC-EXT-006](external-agent-runtime-play.prd.md#ac-ext-006) | §2.3 目标应用与停止后续委托。 |
| [REQ-EXT-007](external-agent-runtime-play.prd.md#req-ext-007) | [AC-EXT-007](external-agent-runtime-play.prd.md#ac-ext-007) | §2.3 回访；§3.1 分开的四类状态。 |
| [REQ-EXT-008](external-agent-runtime-play.prd.md#req-ext-008) | [AC-EXT-008](external-agent-runtime-play.prd.md#ac-ext-008) | §3.2 故障恢复与原世界历史。 |
| [REQ-EXT-009](external-agent-runtime-play.prd.md#req-ext-009) | [AC-EXT-009](external-agent-runtime-play.prd.md#ac-ext-009) | §3.2 预算限制；§5 两类成本与继续选择。 |
| [REQ-EXT-010](external-agent-runtime-play.prd.md#req-ext-010) | [AC-EXT-010](external-agent-runtime-play.prd.md#ac-ext-010) | §2 完整首局经历；§5 两个 Runtime 分别验证。 |
