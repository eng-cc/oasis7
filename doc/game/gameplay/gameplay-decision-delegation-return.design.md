# 经营决策、目标委托与回访续接系统设计

- 状态：`active`（长期设计约定，不代表实现、试玩或发行已完成）
- Owner role：`gameplay_designer`
- 专业协同：`producer_system_designer`、`runtime_engineer`、`agent_engineer`、`viewer_engineer`、`qa_engineer`
- 产品要求：[`首局与持续游玩 PRD`](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md)
- 产品体验方案：[`设计范围与具体体验`](../../product/world-rules-core-gameplay/first-session-and-continuation.design.md#gameplay-experience-priorities)
- 系统边界：本文只细化既有产品要求的组合接线、事实投影、异常与验证。领域规则、授权、报价、结算和持久化仍由相邻专业 authority 拥有。
- 审读范围：对应现有 Smelter 首产物、后继工业分支、间接控制和 Viewer/API 接点；实现状态与实际候选证据从 PR 和验证记录读取，不从本设计的 active 状态推导。

## 1. 问题、目标与非目标

现有动作、分支说明、回执和恢复字段可以分别存在，但玩家仍可能无法比较真实取舍、需要逐步操作、看不懂阻塞，或回来后不知道继续什么。本文把这些接点组织成“比较目标与投入 → 有界委托 → 权威执行 → 结果解释 → 下一决策 → 回访”的组合设计。

当前范围保持 `starter-industrial-smelter-to-assembler-v1` / `production_only`。真实生产结算与首产物里程碑成立后只开放 Assembler 候选；不把建成 Assembler、稳定窗口、真实交易或策略模板增加为首产物条件。首局、首次持续能力、合作交付和真人续玩意愿分别判断。

非目标：不新增经济数值、全知需求服务、万能 Agent、平行回执或授权账本；不改变共识、WASM ABI、生产结算、公开可用性或 CI 门禁；不因为有可用性投影就宣称两条经营路线均有价值。真实合作与策略复用只保留后续设计入口，不在本设计中获得新的执行权限。

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [REQ-FIRST-BRANCH-001](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-branch-001) Trace refs: O7-REQ-4c622b94-22bd-4559-9595-6b134bdc00b1 | 在分支适用阶段比较真实可达动作、投入、替代用途与后续两个决策；不能用标签或参数数量证明分支 | [DES-ADOPT-001](#des-adopt-001) | gameplay 定义路线价值；M4/Runtime 发布事实；Viewer/API 表达 | 不强制首笔生产前出现两条路线，不引入反事实模拟服务 |
| [REQ-FIRST-INDUSTRIAL-002](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-002) Trace refs: O7-REQ-b99516b0-4b36-4aa1-84e4-efc07837cd95 | 同一对象、配方、批量和账本的预览、提交、结果支持玩家决策 | [DES-ADOPT-003](#des-adopt-003) | M4 报价/提交；Runtime 动作；Viewer 展示 | 不在前端复制配方常量或补算权威余额 |
| [REQ-WR-IA-003](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-003) Trace refs: O7-REQ-b38abaf7-824d-47b7-a954-bcdb57897376 | 目标或纠正与 Agent 后续行为及结果可关联；内部重复操作不冒充玩家决策 | [DES-ADOPT-002](#des-adopt-002) | Agent/Runtime 因果链；gameplay 主体感；QA 观察 | 不暴露私有推理，不用目标接受代签世界效果 |
| [REQ-AGENT-AUTH-001](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#req-agent-auth-001) Trace refs: O7-REQ-bd548350-7fce-4026-aeec-5b26a10b86ad | 委托只在当前支持的能力与有效授权交集中执行，累计边界沿原 authority 核验 | [DES-ADOPT-002](#des-adopt-002)、[DES-ADOPT-006](#des-adopt-006) | [Runtime 授权设计](../../world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | 不把偏好、模板、WASM capability 当作玩家授权；不新建跨 grant 总额度 |
| [REQ-FIRST-INDUSTRIAL-005](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-005) Trace refs: O7-REQ-9a0630f6-b261-420b-85a1-1dd426cf3b48 | 漂移、断连、重试、乱序和恢复不复用旧资格、不重做未知提交、不复制结果 | [DES-ADOPT-003](#des-adopt-003)、[DES-ADOPT-004](#des-adopt-004) | M4/Runtime 原请求结果与恢复合同 | 不保证关闭界面会取消提交，不由新快照推断旧请求未生效 |
| [REQ-FIRST-INDUSTRIAL-006](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-006) Trace refs: O7-REQ-d0041aa9-1dfb-46a0-95e0-67417b39bfa3 | 主要阻塞、保留/消耗价值及当前安全下一步可读 | [DES-ADOPT-003](#des-adopt-003) | Runtime blocker；M4 可恢复条件；Viewer/API | 不编造补料、补电、取消或等待恢复能力 |
| [REQ-FIRST-INDUSTRIAL-008](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-008) Trace refs: O7-REQ-757636b1-c9e9-4e7a-97d1-460de105daf3 | 成果、后继用途、自然离场与回访接续保持同一世界、归属和目标语义 | [DES-ADOPT-004](#des-adopt-004) | 领域 receipt、Runtime 恢复、Viewer 投影 | 不按离线时长发收益，不由摘要生成里程碑或责任归因 |
| [REQ-FIRST-INDUSTRIAL-004](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#req-first-industrial-004) Trace refs: O7-REQ-81c0bc21-36ef-400d-9398-31859b9a08b2 | 用途说明、需求事实、生产完成与交付后果分开；production_only 不代签需求满足 | [DES-ADOPT-005](#des-adopt-005) | 现行生产/交付与需求 authority | 不在当前范围实现市场、合同或 terminal-admission 正向链 |

### 2.2 相邻 authority

[产品具体样本](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#first-session-concrete-sample)拥有玩家目标与完成边界；[M4 报价到提交](../../world-simulator/m4/industrial-resource-flow-contract.design.md#starter-quote-submit-binding)拥有实际方案绑定；[未知结果分离](../../world-simulator/m4/industrial-resource-flow-contract.design.md#request-outcome-observation-separation)拥有请求结果与观察状态；[Runtime 间接控制](../../world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-007)拥有因果与纠正接线；[Viewer 当前方案交互](../../world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md#starter-plan-interaction)拥有现有界面接点。本设计不重新定义这些合同。

## 3. 当前状态、目标状态与差距

下列源码只证明对应接点存在；未通过真实入口验证的组合能力保持未核实。固定审读版本、运行状态与测试结果记录在实际 PR，不追加到本表。

| 当前可核对的接点 | 目标状态 | 仍需实现或验证的差距 |
| --- | --- | --- |
| [branch_commitment.rs](../../../crates/oasis7/src/viewer/runtime_live/branch_commitment.rs) 从已发布且未禁用动作筛选分支，包含即时收益、后续步骤与风险说明 | 可执行性与经营价值分开；成本和后继条件来自同次可信事实 | 动作存在不证明两条路线的机会成本和后继两个决策确实不同；需配对场景核验 |
| 同文件的 first_delivery_preview 按 action_id 选择 local_need、expected_output 等文案，required_inputs 来自动作输入辅助函数 | 数量来自实际方案；用途、已确认需求与交付后果有清楚证据层级 | 不能把动作映射字符串当作真实需求、生产数量或已交付证明；具体输出与当前配方一致性待实现验证 |
| [gameplay_snapshot.rs](../../../crates/oasis7/src/viewer/runtime_live/gameplay_snapshot.rs) 组合 available_actions、阶段、反馈、阻塞及因果信号 | Viewer/API 使用一致上下文解释决策、阻塞和结果，不在展示端另算世界状态 | 字段存在不证明预览、请求、结果和回访已形成可用组合；跨入口、迟到结果与缺失证据需单独验证 |
| Runtime 已有间接控制、授权、因果与恢复设计 | 一条受支持生产链可在有效授权内代办，并在真正需要新判断时请示 | 逐动作的 Agent 接线、强约束执行与体验证据不能由设计存在推出 |
| 当前产品设计已经区分成果、因果贡献和后继用途 | 回访聚合事实，但不颁发第二份 receipt、生成离线收益或漏掉待决义务 | 回访窗口完整性、读状态与跨会话投影需专门验证 |

## 4. 边界与结构

采用既有链路上的薄投影，不引入新的世界 authority 或独立编排服务：

`玩家目标/约束 → Agent 计划或玩家基线操作 → 既有 Runtime 授权与提交 → M4/领域结算 → 既有 gameplay/receipt 投影 → Viewer 与 pure API`。

- gameplay 拥有分支的产品含义和可比条件；不能靠改文案弥补不成立的规则取舍。
- Agent 负责在已支持的动作和授权内安排步骤；控制权、额度、资源与世界效果仍在权威执行边界判断。
- Runtime/M4 负责权威事实、报价、原请求身份、执行、结算和恢复；既有账本是唯一经济真值。
- 投影层复用当前动作、blocker、因果链与 receipt，输出阅读模型。Viewer/API 消费同义事实，不分别猜测资格或成本。
- 记忆、自然语言摘要与展示缓存只能引用可见事实；不能写回世界状态、决定授权或补齐缺失因果。

## 5. 关键运行流程

<a id="des-adopt-001"></a>
### DES-ADOPT-001：经营分支比较

先在当前阶段和可见范围内获取规范动作集合，再取匹配对象、配方、批量、账本与规则版本的报价和后继条件。一个候选只承诺它已有证据的部分：立即可做、前置未满足但可查看，或事实未确认，不能混为一个可提交按钮。

以既有“深化 Smelter”和“建设 Assembler”为首个比较对象，分别检查本次真实消耗、执行后可用余量、占槽/时机、形成能力和后续两个可达决策。未实现报价的量保留未知；不从 expected_output 或 risk 文案反向生成权威数值。后继步骤是条件性路径说明，不是预测收益保证。

机会成本用“该资源或时段还能做什么”表达。可复现的规则对照使用同一初始世界状态分叉，在各自合法路径上核对差异；这是测试方法，不是在正式世界复制资源或增加运行时反事实引擎。小批/大批只有在材料、电力、实际时间、占槽、时机与替代用途合并核对后才可能构成分支。

当前只有一条合法路线时，保留真实操作与未满足前置，不伪造第二项，也不宣布分支验收完成；后续阶段的原分支义务仍保持未满足。选择后按当前状态重新验证，不能把比较时的资格当作锁定额度或预留产能。

<a id="des-adopt-002"></a>
### DES-ADOPT-002：目标级委托与请示

先区分三种事实：玩家想达到的目标、仅影响 Agent 选择的软偏好、由现有执行路径强制核验的授权边界。没有强制执行接线的限制不能写成系统保证；缺少适用控制权或动作接线时，保留玩家操作基线，不能包装为自主代办。

| 发生的变化 | 处理方式 | 事实与授权边界 |
| --- | --- | --- |
| 当前能力、预算与授权内的重复检查或合法执行步骤 | Agent 可继续代办，保留可查状态，不为内部检查重复索取玩家确认 | 每次实际提交仍走原权限、累计额度与资源检查；减少弹窗不减少校验 |
| 可恢复 blocker，且恢复手段已在授权范围内 | 只采用当前受支持恢复路径，显示原因与复查条件 | 不能因要完成目标就增加购买、建厂、转账、运输等未授权动作 |
| 需要扩大目标、权限、累计承诺，或作出原授权未覆盖的不可逆取舍 | 在新增动作提交前请示，说明当前已保留/消耗价值及替代 | 不把已授权的同类成本每次重问；也不由自然语言目标推导新增权限 |
| 授权失效、控制权变化或事实未确认 | 停止新增越界动作；核对已有请求，再给重新授权、缩小范围或安全停止等真实选项 | 不复活旧授权，不追溯取消已结算结果 |
| 提交结果未知 | 沿原请求查询或按原协议安全恢复 | 不换 request identity 再做一遍，不把更换目标当取消 |

相同未变化的请示可以在表现层合并；合并不丢掉原请求和风险。用户作出新决定后，旧请示失效或归档，但迟到结果仍归原请求。目标、软偏好或纠正如何影响后续动作，通过现有 intent/action/result 因果链证明，不以“Prompt 更新成功”代替。

<a id="des-adopt-003"></a>
### DES-ADOPT-003：决策面、阻塞与结果

当前决策面只组织四类内容：目标与实际方案、本次投入及可见剩余约束、主要 blocker、一个主动作和可展开替代。涉及权限、不可逆损失或承诺扩大时，重要信息优先于普通生产说明；详细账本仍可回看。显示“预计”与“已结算”时使用明确不同语义，不用动画、按钮状态或计数制造完成。

预览、确认、提交和结果必须关联同一方案。选择、参数、账本、规则版本、权限或可执行性改变后，旧报价只能作为历史参考；重新获得匹配报价并走权威重验，不能静默改配方、批量或消费账本。乱序响应按所属上下文保留或舍弃展示，不能覆盖新选择；原请求的结果核对不因舍弃旧展示而丢失。

阻塞投影分别提供原因、受影响目标、已保留/占用/消耗/损失价值，以及目前确实支持的下一步。缺值为未知，不填零或“无需成本”。多个原因按原安全/授权优先级组织，不能为了只展示一个主 blocker 而隐藏另一个不可逆风险。

Viewer 的设施关联、输入/产出联系只表现真实依赖；当前配方不依赖物流或多输入，就不渲染虚构运输过程。pure API 可以结构化表达，但不能多获得隐藏库存、需求或更宽权限。

<a id="des-adopt-004"></a>
### DES-ADOPT-004：成果与回访摘要

成果展示引用领域结算和持久里程碑；玩家贡献额外要求原意图、动作或 job 与该结果有精确关联。只有 owner 相同、材料同名或时间接近时，只能说是该主体的成果，不能说是当前决定造成。复用既有 Action Receipt 的唯一展示责任；摘要是链接与解释，不新发回执或另存一份可结算结果。

回访按以下次序处理：确认当前世界/归属与可读状态；读回已结算成果；核对原目标与尚未确认请求；重验授权和当前能力；最后给出合法下一步。已结算、仍待决、状态不明与仅属后继候选分别呈现。

增量摘要只在连续且可验证的原事件窗口上声称“自上次查看之后”。窗口缺失、cursor 不兼容、重组、世界/归属改变或源不可用时，降级为“截至当前已确认状态”，标明缺口并复核原请求；不得用两个库存快照的差值推断生产、交易或因果。没有执行证据时，不按墙上时钟推算离线产量；服务未运行、无新事件和无法确认是三种不同状态。

按原事件身份去重，更新展示的已读位置不改变经济账本、目标完成或请求状态；读取摘要失败不能阻塞生产。普通环境变化可以合并，损失、授权变化和待决义务不能被摘要裁剪为不存在。回访中的下一动作仍须提交时重验，关闭界面不撤销原请求。

<a id="des-adopt-005"></a>
### DES-ADOPT-005：用途与真实需求分层

当前就需要修正的是表达真值，而不是新增市场：

| 证据范围 | 可以表达 | 不得推导 |
| --- | --- | --- |
| 只有配方、科技或后继能力关系 | 潜在用途、自己的后继目标及实际缺口 | 某个邻居、区域或组织正在等待供货 |
| 有当前可见、有效且有明确需求方的需求记录 | 已确认需求及该记录支持的条件、未满足量和复查边界 | 已签约、已成交、保底利润或需求永远有效 |
| 有已接受的合作承诺 | 对应义务及尚未完成范围 | 已生产、已交付或需求已经减少 |
| 有匹配的交付/领域结算与目的地后果 | 实际交付范围及可证明的需求变化 | 以 production receipt 代签 delivery，或把部分交付说成整体完成 |

first_delivery_preview 的兼容字段名不赋予交付语义。实现前先沿既有字段澄清文案与证据；需要增加证据分类时由既有生产者/消费者兼容合同承接，不直接改名破坏旧快照。没有真实需求证据就退回用途说明，不能用 LLM 补出需求方或报价。

后续最小合作样例可限定一个需求方、一种材料或服务、一项可核对交付结果；需求取消、缩减、到期或未知时，未履行部分沿[需求变更合同](gameplay-industrial-demand-change-disposition-contract.prd.md)处理。该正向链只有在对应产品范围与专业合同明确采用后实施，不反向增加当前首产物门槛。

<a id="des-adopt-006"></a>
### DES-ADOPT-006：策略复用的后续边界

后续优先复用现有目标配置、参数或已支持模块，而不是新建策略执行引擎。模板仅携带目标、适用条件、可解释参数及来源版本等非授权信息；已有 grant、密钥、控制权、账户余额、原请求身份和未决承诺不能随模板复制。导入内容是待核验输入，不是可信指令来源。

“保存”无世界效果；“再次采用”必须在当前对象、能力、版本和授权下重新预览并形成新的合法意图，不能重放上次提交。历史成功仅作历史证据，不保证相同收益。WASM 模块仍服从其现有发布、许可、资源与升级合同，不把模板功能变成新的 capability grant。

本次不冻结模板 schema、存储位置、市场、分享权限或发布渠道。由 producer/gameplay 与 Agent/runtime/WASM owner 在存在可重复的真实玩法、明确采用入口和兼容范围后细化；未完成这些决策时不显示可用的保存/导入/执行按钮。

## 6. 接口与数据合同

下表是读模型必须保留的逻辑事实，不是新 API、DTO 字段清单或既有字段存在声明。优先映射原合同；原生产者缺少所需事实时，标记缺口并由其 owner 补齐，不由 Viewer 推断。

| 逻辑事实 | 唯一生产者/依据 | 消费与缺失语义 |
| --- | --- | --- |
| 读取上下文：世界、归属、可见范围、规则/快照新鲜度 | 既有 Runtime/会话 authority | 只组合兼容上下文；缺失或冲突时不产生猜测性写动作 |
| 方案：动作、对象、配方、批量、消费账本、报价条件 | 规范动作来源及 M4 | 展示和提交保持同一绑定；没有权威报价不从字符串计算成本 |
| 意图、授权、实际动作与原请求关系 | 既有 Agent/Runtime 因果与授权合同 | 软偏好、有效授权、请求接受和实际效果分别表达 |
| 消费、保留、产出、交付与结果身份 | 领域账本、receipt 和原请求 disposition | 各自只证明其声明范围；缺证据保持未知 |
| 阻塞、恢复条件、可用动作 | 当前权威快照及专业恢复合同 | 当前不可用的动作不能包装成恢复按钮 |
| 后继用途、需求与持续能力 | 对应玩法规则或真实需求 authority | 潜在用途不等于需求，需求不等于合作已完成 |
| 回访阅读窗口与已读位置 | 原 journal/receipt 身份及非权威显示状态 | 只控制重读和摘要；不触发结算或增加离线进度 |

不要求消费端读取私有 Prompt、完整模型推理或不属于其可见范围的数据。自然语言只解释已发布事实，不能决定请求结果或提升缺失数据的置信度。

## 7. 状态、事务与持久化

不新建世界状态机。目标接受、动作提交、执行/阻塞、结果未知、领域结算与玩家已读沿原 authority 各自表达；它们不因组成一个页面而合并为一个 completed 状态。

预览和摘要为只读；实际资源检查与扣减、原请求去重及结果持久化继续在现有权威提交边界执行。并发 Agent、玩家操作或重试不得绕过累计额度，也不由前端余额预扣代替领域事务。

展示缓存须隔离世界、归属与可见范围；上下文或授权变化后失效。仅已读位置和展示偏好可以独立保存，不能与生产提交组成新事务。读取位置丢失可重新展示历史摘要，但不能重复播报为新成果或重复结算。原始事件不可获取时只报告可证明的当前事实和未知范围。

## 8. 部署、安全与运行约束

本设计不新增服务、开放端口、模型提供商、凭据或生产权限。Viewer/API 使用已有认证和可见性过滤；公开需求与私有经营记录按原 authority 区分。

Agent 的目标、收到的消息和导入模板均不覆盖系统授权。没有强制预算校验时不宣称硬限额；不得为演示自动化而使用 debug 注入、免费输入或更高权限。停止后续代办与取消已提交请求分别处理，失效授权不能阻止合法查询原请求已经发生的结果，但查询本身仍遵循当前读取权限。

## 9. 质量与容量

比较与摘要优先使用现有 snapshot、quote、journal 和 receipt，不为每次绘制遍历全世界或重新调用模型。沿已有事件分页、保留和可见性界限读取；缓存键覆盖来源版本与上下文，不能只按 action_id 缓存可执行性。

确定性事实摘要应有不依赖 LLM 的路径。可选语言润色失败时退回事实，不阻断操作、结算或回访。性能验证观察读取次数、投影耗时、缓存失效、摘要窗口与界面可交互性；数值预算由受影响专业 owner 依据实际场景设定，不在本文虚构已达成指标。

## 10. 兼容、迁移与回滚

先在现有读模型和界面内改善内容及来源绑定；只有事实确实无法表达时才扩展原生产者合同。扩展需核对序列化、旧快照、Viewer、pure API、Agent 与测试消费者；缺字段不能默认成已授权、已交付或零成本。本文不冻结新的 wire schema 或代替该兼容评审。

无法可靠生成新比较或摘要时，回退到原有真实动作、权威状态与 receipt，不回退为乐观猜测。禁用新增表现不改变已经提交或结算的世界效果，不删除原始证据。既有 production_only 与 terminal-admission 的边界不随展示回退改变。

## 11. 验证设计与可追溯性

各场景分别记录固定候选、实际入口、世界/归属、profile、可用动作、权限、主要事实与未覆盖范围。优先验证一个 Smelter 样本的纵向链，再验证适用阶段的分支；不把后续合作和模板候选加入当前首产物通关清单。

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-FIRST-BRANCH-001](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#ac-first-branch-001) Trace refs: O7-REQ-4c622b94-22bd-4559-9595-6b134bdc00b1 | [DES-ADOPT-001](#des-adopt-001) | 可达性、真实取舍及后继两个决策独立成立 | VAL-ADOPT-001：按[自测手册](../../testing/manual/agent-runtime-self-playtest.manual.md)在相同初始状态的两个合法样本中比较深化 Smelter/建设 Assembler；覆盖仅一条可用、旧候选失效和小批/大批等价反例。规则对照与真人解释分开 | PR 中的两条世界结果、成本/用途对照及玩家观察 | 不以两张卡片、不同文案或 fixture 证明真人策略深度 |
| [AC-WR-IA-003](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#ac-wr-ia-003) Trace refs: O7-REQ-b38abaf7-824d-47b7-a954-bcdb57897376 | [DES-ADOPT-002](#des-adopt-002) | 一个支持的目标/约束变化影响 Agent 后续选择并可归因 | VAL-ADOPT-002：按[自测手册](../../testing/manual/agent-runtime-self-playtest.manual.md)保留其他主要条件，只改变一个已支持约束，核对实际动作/结果；记录人工提示和重复确认。未接入步骤单独标明 | PR 中 intent/action/result 链与提示记录 | Agent 自报、Prompt ACK、生产成功不证明主体感或自主理解 |
| [AC-AGENT-AUTH-001](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-agent-auth-001) Trace refs: O7-REQ-bd548350-7fce-4026-aeec-5b26a10b86ad | [DES-ADOPT-002](#des-adopt-002)、[DES-ADOPT-006](#des-adopt-006) | 自动执行不越权，模板不复制授权 | VAL-ADOPT-003：按[测试手册](../../../testing-manual.md)选择 Runtime/Agent 授权场景，覆盖累计耗用、并发、重试、过期/撤销与软偏好不等于硬限制；模板真正进入采用范围后再测保存无效果、重新授权及旧版本不兼容 | PR 中授权/请求/领域结果及被拒绝范围 | 当前文档不证明模板功能已实现；不新增跨 grant 额度规则 |
| [AC-FIRST-INDUSTRIAL-003](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#ac-first-industrial-003) Trace refs: O7-REQ-9a0630f6-b261-420b-85a1-1dd426cf3b48 | [DES-ADOPT-003](#des-adopt-003) | 同方案、漂移重验、未知结果与去重 | VAL-ADOPT-004：按[测试手册](../../../testing-manual.md)在 quote→submit→receipt 链改变对象/批量/权限或账本条件，加入旧响应迟到、响应丢失但已生效、部分结算和并发；核对原请求只有原效果 | PR 中报价绑定、原请求 disposition 与账本/receipt | UI 单测不证明真实 Runtime；刷新成功不证明未知结果已核对 |
| [AC-FIRST-INDUSTRIAL-002](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#ac-first-industrial-002) Trace refs: O7-REQ-d0041aa9-1dfb-46a0-95e0-67417b39bfa3 | [DES-ADOPT-003](#des-adopt-003) | 主要 blocker、价值与安全下一步可理解 | VAL-ADOPT-005：按[自测手册](../../testing/manual/agent-runtime-self-playtest.manual.md)覆盖缺料、低电、权限变化、多个 blocker、无安全路径、源缺失；Viewer/pure API 分别读取并核对事实同义，人工观察理解迁移 | PR 中两入口输出、世界结果与提示前后观察 | 不强迫所有真人首局经历故障；自动化不代签理解 |
| [AC-FIRST-INDUSTRIAL-004](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#ac-first-industrial-004) Trace refs: O7-REQ-757636b1-c9e9-4e7a-97d1-460de105daf3 | [DES-ADOPT-004](#des-adopt-004) | 成果、义务与回访的下一决定保持连续 | VAL-ADOPT-006：按[自测手册](../../testing/manual/agent-runtime-self-playtest.manual.md)在首笔匹配结算后自然离场，覆盖无新事件、服务未运行、事件窗口缺口、重复/迟到结果、旧授权失效和世界切换；独立记录主动回来后的第一动作 | PR 中原 receipt、重入事实、窗口边界与真实玩家观察 | 合成摘要、Agent 继续执行或自动重连不证明真人回访/留存 |
| [AC-FIRST-INDUSTRIAL-005](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md#ac-first-industrial-005) Trace refs: O7-REQ-81c0bc21-36ef-400d-9398-31859b9a08b2 | [DES-ADOPT-005](#des-adopt-005) | 用途、需求、合作承诺和交付严格分层 | VAL-ADOPT-007：按[测试手册](../../../testing-manual.md)核对仅有静态 local_need 文案、真实需求未知/取消、仅 production receipt 和部分交付四类反例；当前 production_only 可完成而不减少需求、不发交付奖励 | PR 中来源分类、可见文案与领域结果 | 无需先实现完整市场或 terminal-admission 正向链；需求存在不证明盈利 |

上述 VAL 名称是本设计的场景标识，不冒称已有同名自动化用例。[吸引力自动化入口](../../../scripts/verify-gameplay-attraction-automation.sh)及专业手册继续区分结构、真实运行和真人证据。文档链接/结构检查、规则回归、Agent 实玩、真人理解与留存不得互相代签。

## 12. 决策、长期风险与未决问题

| 决定或未决项 | 采用方案与理由 | owner 与复核触发 |
| --- | --- | --- |
| 承接既有要求而非新增六个模块 | 在同一工业链上组合决策、委托、解释和回访；复用原 Trace ID，不另立产品或执行 authority | producer/gameplay；主目标或完成 profile 改变时复核 |
| 两条路线是否真正有价值 | 先比较现有深化 Smelter/建设 Assembler；缺少真实差异时保持单路线基线和分支缺口，不靠奖励或强制事故补齐 | gameplay；同初始状态、实际时长/占槽与后继用途对照成立后采用 |
| 当前哪些步骤可由 Agent 代办 | 逐动作核对现有接线、权限与因果，软偏好不升级硬保证 | Agent/runtime；对应动作从玩家基线转为自动执行前核对 |
| 回访窗口是否完整 | 优先原 journal/receipt；不完整时显示当前已确认事实而非虚构增量 | runtime/Viewer；cursor、恢复、重组、保留期限或权限变化时复核 |
| 何时引入真实合作 | 当前先纠正用途/需求表达；真实需求方、合法交付与失败处置具备后，由原工业/市场专题承接 | producer/gameplay/M4；不作为首产物、邀请测试或本设计通过的新增条件 |
| 何时引入策略复用 | 先证明有值得重复的有效玩法，再选择现有配置/模块的最小复用入口；保存与授权分离 | producer/Agent/runtime/WASM；对象/版本/权限和兼容方案明确后细化 |
| 可读摘要误导或泄露 | 确定性事实优先、可见性先过滤、因果不足不补全；语言润色不参与执行 | Viewer/runtime/QA；任何事实来源或摘要组件变更时复核 |
