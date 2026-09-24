# Indirect Control Agency Execution and Continuation — Runtime System Design

- 设计 ID：`DES-WR-IA`
- 状态：`active`
- Owner role：`runtime_engineer`
- 专业共同审读：`gameplay_designer`、`runtime_engineer`、`agent_engineer`、`viewer_engineer`、`qa_engineer`
- 对应专业需求：[`PRD-GAME-014`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#1-executive-summary)
- 上游产品要求：[`REQ-WR-IA-001`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-001)、[`REQ-WR-IA-002`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-002)、[`REQ-WR-IA-003`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-003)
- 补充产品 authority：[`Agent 自治、委托与责任连续性`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#agent-delegation-boundary)；本设计只承接其技术边界，不取代该 PRD、资产经济 authority 或玩法/Viewer 专业合同。
- 审读基线：本文只定义长期系统设计与验证方法；实现现状、候选提交、测试结果和任务状态必须从关联 GitHub Task UID 的 issue evidence 读取。
- Last reviewed：2026-09-16

本文把玩家意图、Agent 解释、runtime 权威执行、Viewer/API 投影和 QA 证据组织成一条可追踪的间接控制因果链。产品 PRD 拥有玩家承诺与 REQ/AC，`PRD-GAME-014` 拥有玩法 guarantee、字段语义与失败签名，本文拥有跨组件技术边界、状态映射、恢复合同和验证设计；它不复制产品要求、任务台账或当前完成度。

补充的 Agent authority 承接范围是授权有效性与范围检查、控制权变化后对未生效行动的重新评估，以及异议/override 到权威结果的因果归因。玩家/组织授权不等同 WASM capability grant；本文不引入授权字段、签名、receipt schema、界面或资产转让规则。

## 1. 问题、目标与非目标

### 1.1 问题与目标

间接控制同时跨越产品、gameplay、runtime、Agent、Viewer/API 与 QA。若缺少系统设计层，产品要求只能直接跳到实现或任务，容易出现 `accepted` 被误写成 `applied`、各入口自行推导状态、回流只恢复日志、记忆纠正无法关联后续结果等漂移。

本设计目标是：

- 用稳定的 `DES-WR-IA-*` 条款承接产品 REQ/AC 与 `PRD-GAME-014`；
- 让 Agent 自治、授权、转让后重配置、异议/override 与责任连续性产品条款拥有明确的 runtime 设计与验证映射；
- 明确每个事实的 producer、authority、consumer、持久化与恢复边界；
- 让 Viewer 与 pure API 从同一权威事实投影四类 invariant：accepted intent、execution status、primary reason、next step；
- 把要求、设计条款、验证方法和实际 GitHub Task evidence 串成可查询链路。

### 1.2 非目标

- 不改变产品的间接控制方向，不新增第一人称逐帧控制承诺。
- 不在本文冻结具体 UI 布局、API payload、runtime enum、Agent prompt/model/provider 或记忆算法。
- 不把 Agent 输出、请求接受、队列入列、界面计数或 world tick 单独当作权威世界效果。
- 不记录排期、分支、HEAD、CI 结果、当前 verdict 或发布状态；这些属于 GitHub task truth 与 QA evidence。
- 不以文档建档证明实现完成、跨入口 parity 或 release readiness。

### 1.3 裁剪说明

本主题跨 runtime、Agent、Viewer/API、持久化、恢复与验证边界，适用完整十二段系统设计视图。安全与容量不新增数值目标，但保留权限、隐私、资源与未验证边界。

## 2. 上游约束与相关角色

### 2.1 需求承接与分配表

| 上游 requirement / acceptance | 具体 obligation | 本设计条款 | 外部 owner / dependency | 明确排除 |
| --- | --- | --- | --- | --- |
| [`REQ-WR-IA-001`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-001) | 接受与权威世界生效保持可区分 | [DES-WR-IA-001](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-001) | gameplay / runtime / Agent | 不规定具体 schema 或 UI |
| [`REQ-WR-IA-002`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-002) | 阻塞、离开与回流仍保留有效下一决策 | [DES-WR-IA-003](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-003) | runtime / Agent / Viewer | 不承诺所有意图都可恢复 |
| [`REQ-WR-IA-002`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-002) | 离开、重连与回流从权威证据恢复或明确重新决策 | [DES-WR-IA-006](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-006) | runtime / Agent / Viewer | 不承诺失效意图继续恢复 |
| [`REQ-WR-IA-003`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-003) | 关键决策的理由、stakes、替代、纠正和结果处于同一因果链 | [DES-WR-IA-007](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-007) | Agent / runtime / Viewer / QA | 不暴露私有 prompt 或内部 trace |
| [`AC-1`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-1) | 高自治与有界授权都是对 Agent 工作方式的选择，不得改变统一世界规则、资源、权限与治理边界 | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | Agent / runtime；模式语义由产品 authority 拥有 | 不新增 runtime enum 或系统权限 |
| [`AC-2`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-2) | 高后果行动须有仍有效且范围匹配的预授权或有效确认；超范围、失效或硬阻断时不得产生世界效果 | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | Agent / runtime；硬边界由对应 authority 拥有 | 不规定 authorization schema、字段或签名 |
| [`AC-3`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-3) | 团队扩张不能静默扩大自治范围或责任；具体资产取得、维护与协调约束仍由 gameplay authority 提供 | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | Gameplay / Agent / runtime | 不定义团队经济、容量或平衡 |
| [`AC-4`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-4) | 转让后新 owner 的权限从专业生效边界开始；Agent 身份、来源和已生效历史继续可追溯 | [DES-WR-IA-011](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-011) | Gameplay / runtime / Agent | 不定义转让市场、条件或价格 |
| [`AC-5`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-5) | Agent 异议、世界/安全硬阻断与有效 owner override 在权威结果中可区分 | [DES-WR-IA-012](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-012) | Runtime / Agent / Viewer / QA | override 不越过硬边界；不规定 receipt 字段 |
| [`AC-6`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-6) | 高影响结果保留 Agent、owner、组织策略和执行载体之间适用的责任因果关联 | [DES-WR-IA-012](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-012) | Runtime / Agent / QA | 不定义法律责任制度或暴露私有 prompt |
| [`AC-7`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-7) | 工业供给或治理 quota 不能静默扩大授权与责任边界；资源/产能合同由 gameplay authority 提供 | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | Gameplay / runtime / Agent | 不定义工业经济、额度公式或平衡 |
| [`AC-8`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-8) | 授权变化、转让生效或新硬边界后，未生效请求必须重新评估或明确终止，不得静默续行、重放或改写已生效结果 | [DES-WR-IA-011](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-011) | Runtime / Agent / gameplay / QA | 不定义具体状态枚举或重放协议 |
| [`AC-9`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-9) | 高后果预算/数量上限在其有效期内累计检查；拆分、并发、重试、重连或主体切换不能复制或重置额度 | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | Runtime / gameplay / Agent | 不定义预算字段、来源规则或经济公式 |
| [`PRD-GAME-014 causal-decision receipt`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#21-reusable-causal-decision-receipt) | gameplay receipt 语义由可验证系统边界承接 | [DES-WR-IA-009](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-009) | gameplay_designer | 本文不重新拥有字段表和失败裁决 |
| [`PRD-GAME-014 causal-decision receipt`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#21-reusable-causal-decision-receipt) | receipt、记忆纠正与最早生效点沿同一因果链承接 | [DES-WR-IA-007](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-007) | Agent / runtime / QA | 不暴露私有 prompt 或内部 trace |
| [`PRD-GAME-014 causal-decision receipt`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#21-reusable-causal-decision-receipt) | receipt 在 Viewer/API 以同源 invariant 投影并受权限/隐私约束 | [DES-WR-IA-008](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-008) | Viewer / runtime / QA | 不冻结相同 payload 或布局 |

角色边界：`producer_system_designer` 维护跨域取舍与产品授权/责任语义；`gameplay_designer` 维护 guarantee、资产经济和失败签名；`runtime_engineer` 维护 canonical 状态、校验、提交、receipt、回放和恢复；`agent_engineer` 维护解释、计划与记忆处理；`viewer_engineer` 维护玩家表面投影；`qa_engineer` 维护实际候选的验证与阻断判断。组织代表权和 Agent 授权的具体凭据/规则由其专业 owner 决定，不从 WASM module grant 推导。

## 3. 当前状态、目标状态与差距

| 对象/能力 | 当前状态（基线） | 目标状态 | 差距/假设 | 证据或 owner |
| --- | --- | --- | --- | --- |
| 产品/玩法要求 | 已有产品 REQ/AC 与 `PRD-GAME-014` guarantee、字段和失败签名 | 稳定映射到设计条款 | 旧 design 缺少系统边界与验证映射 | product PRD / gameplay PRD |
| runtime authority | 已有 Agent cognition 生命周期设计，声明 runtime 拥有 canonical transition，Agent 输出是 data | 所有 agency 投影都引用权威状态与 receipt | 各具体玩家意图路径的接线需逐候选验证 | [`agent-cognition-lifecycle.design.md`](../../world-runtime/runtime/agent-cognition-lifecycle.design.md#1-boundary-map) |
| Agent 解释与记忆 | 存在专业合同；当前实现覆盖不得从本文推断 | 理由、记忆使用、纠正与 earliest effect 接入统一 receipt | 具体意图类型和持久化能力 pending | agent_engineer / task evidence |
| Viewer / pure API | gameplay PRD 定义 parity 地板；当前字段覆盖不得从本文推断 | 两入口表达同一四类 invariant | 具体 transport 与入口 coverage pending | viewer_engineer / task evidence |
| Agent authority 产品追踪 | 产品 PRD 定义授权范围、控制权变化与责任语义；原有 runtime 设计覆盖通用间接控制/receipt | DES-WR-IA-010..012 将产品 authority 条款连到 runtime 校验、待决行动复核和可审计因果结果 | 实现和对应候选测试仍未由本文证明 | `doc/product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md`; runtime / Agent owners |
| 验证 | PRD 已定义 required/full 方向 | 每个 DES 条款有稳定验证方法，实际结果挂 Task UID | 同候选组合证据 pending | qa_engineer / GitHub task evidence |

“目标状态”是设计合同，不等于当前实现。只有固定候选、环境与实际 evidence 同时存在时，某条能力才可被报告为 proven。

### 3.1 已核对的当前 runtime 事实

- 当前 canonical intent 是 [`AgentIntentV2`](../../../crates/oasis7/src/runtime/agent_cell.rs)，由 [`DomainEvent`](../../../crates/oasis7/src/runtime/events/domain_event.rs) 驱动 proposal、submission、acceptance、transition 和 replacement；具体状态与校验仍以源码和 runtime authority 为准。
- retry identity、request digest 与 replacement 校验见 [`agent_intent.rs`](../../../crates/oasis7/src/runtime/world/agent_intent.rs)：身份绑定玩家、Agent、世界、reorg、authority 和 replacement 上下文；相同 durable digest 可重放，冲突 fail closed，新意图替换 active intent 时必须精确引用被替换 intent。
- completion 校验见 [`agent_intent_terminal.rs`](../../../crates/oasis7/src/runtime/world/agent_intent_terminal.rs)：需要 accepted intent、完整 authority tuple、绑定 effect intent 以及先前已提交的匹配 receipt；provider/UI ack 不能产生 completion。
- 当前 Viewer 投影见 [`gameplay_snapshot.rs`](../../../crates/oasis7/src/viewer/runtime_live/gameplay_snapshot.rs)：`accepted_intent_id` 可能来自 player-facing feedback action，不能与 canonical `AgentIntentV2.intent_id` 或 receipt identity 互换；任何映射都必须显式、版本化并验证。
- Viewer 与 pure API 的共同投影骨架见 [`PlayerGameplaySnapshot`](../../../crates/oasis7/src/simulator/persist.rs) 与 [`oasis7_pure_api_client.rs`](../../../crates/oasis7/src/bin/oasis7_pure_api_client.rs)。Web 本地 queued/error/gate feedback 只能作为 provisional overlay，不能覆盖 runtime snapshot 的 accepted/applied/world-effect 真值；`logicalTime`、`eventSeq` 也只能由 runtime snapshot/event 推进。
- 当前 runtime-facing receipt tuple 见 [`control_blocking.rs`](../../../crates/oasis7/src/viewer/runtime_live/control_blocking.rs)，尚不包含 IA-003 所需的完整 stakes、alternative、correction outcome 等解释；[`world-runtime design`](../design.md#5-current-implementation-boundary-and-target-gap) 也把统一 execution transaction、持久化、receipt/outbox 的部分能力标为 partial/target。因此本文把完整 causal-decision receipt 写为目标组合合同，而不是当前 proven capability。

## 4. 边界与结构

```text
owner / organization authority
  -> runtime validation + canonical transition (提供授权依据，不直接提交世界)
player intent
  -> Viewer / pure API request surface        (提交与投影，不拥有世界真值)
  -> Agent interpretation / plan              (advisory data，不拥有世界提交)
  -> runtime validation + canonical transition (唯一权威接受、拒绝与应用边界)
  -> world state / journal / receipt           (可回放、可恢复的事实)
  -> Viewer / pure API agency projection       (同源表达)
  -> player interrupt / correction / next decision
```

<a id="des-wr-ia-001"></a>
### DES-WR-IA-001：accepted intent 身份与接受/生效边界

每个受支持的玩家意图必须有可关联的身份。入口可以确认请求已收到或已进入权威处理，但只有 runtime authority 能证明校验、提交或拒绝；`accepted`、`applied`、`persisted`、`published` 必须分别由对应 receipt/state 证明，不能互相推导。Agent 的解释与建议是输入数据，不得直接成为 world authority。

当前实现中的 canonical intent identity、player-facing feedback action 与投影字段可能是不同身份；消费者必须保留显式映射，不得因为字段名包含 `intent` 就视为同一主键。

<a id="des-wr-ia-002"></a>
### DES-WR-IA-002：execution status、主因果与世界效果

agency surface 必须把玩法语义映射到当前 runtime authority，而不是在本文发明新枚举。`overridden`、`completed_no_progress`、`completed_with_progress` 等 gameplay 分类可以由权威 disposition、世界差异与 receipt 组合推导，但映射规则必须版本化、可测试，并保留原始 authoritative disposition。无可验证世界差异时不得报告 progress。

<a id="des-wr-ia-003"></a>
### DES-WR-IA-003：bounded response、阻塞与 fallback

阻塞事实、复查触发与恢复资格由权威组件产生。表面在 bounded-response 窗口后必须从弱等待升级到 `stalled`、`blocked`、`reprioritize_recommended` 或 `fallback_ready` 等玩法分类；`safe_wait` 必须绑定可观察 trigger/recheck，条件失效后重新分类。没有安全 fallback 时必须返回新的决策面，不能形成静默死端。

<a id="des-wr-ia-004"></a>
### DES-WR-IA-004：interrupt、reprioritize 与意图交接

新意图替换、裁剪或改道旧意图时，交接必须保留旧意图引用、请求者、原因、接受结果和 earliest effective point。提交 interrupt/reprioritize 只证明请求已进入处理；只有权威结果能证明旧意图已停止或新意图已生效。并发冲突按 runtime authority 拒绝、排序或 supersede，入口不得私自选择赢家。

<a id="des-wr-ia-005"></a>
### DES-WR-IA-005：有界后果可读性

对每个关键结果，投影层必须能从权威事实解释 `cost`、`progress/world change`、`primary reason` 与 `next step`。这是一张足够支持下一决策的摘要，不是完整预测器；原始日志、调试字段和次级事件不能取代主因果，也不能遮蔽无进展。

<a id="des-wr-ia-006"></a>
### DES-WR-IA-006：resume anchor 与回流恢复

回流恢复以可验证 snapshot、journal、receipt 或 continuation identity 为输入，重建最近有效意图、主阻塞、最近后果和下一步。旧意图失效时必须带原因进入重新决策；raw history 只能辅助审计，不能成为恢复 agency 的唯一入口。恢复不得把未提交的本地/UI 状态升级为 canonical truth。

<a id="des-wr-ia-007"></a>
### DES-WR-IA-007：causal-decision receipt 与记忆纠正

记忆驱动、社交、治理和冲突决策复用 `PRD-GAME-014` 的 causal-decision receipt。Agent authority 决定记忆如何选择、纠正和解释；每个认知 turn 使用带 revision、scope 与 digest 的不可变 `MemoryContextSnapshot`，空 memory 也必须是显式快照。`MemoryWriteIntent` 只是 advisory intent，只有匹配的 committed runtime receipt 才允许提交 authoritative memory；timeout、stale、rejected 或 wait 不得提交。runtime authority 决定相关动作何时被接受和应用；correction receipt 必须关联 correction identity、接受结果、影响范围、earliest effect 与 stale/ignored reason。纠正被接受不等于下一世界动作必然改变。

<a id="des-wr-ia-008"></a>
### DES-WR-IA-008：Viewer / pure API parity、权限与隐私

Viewer 与 pure API 可以使用不同布局和 payload，但对同一候选、身份与时间点，必须从相同 `WorldSnapshot.player_gameplay` 权威投影表达 accepted intent、execution status、primary reason、next step。Web 本地 queued/error/gate 状态只能是带来源和 freshness 的临时 overlay；ack arrival、reload、timer、空 mailbox 或 local snapshot 不得推进 `logicalTime`/`eventSeq`，也不得推导 applied/completed。入口只显示调用者有权访问的脱敏意图与记忆摘要，不暴露其他玩家私有信息、prompt 全文或内部 chain trace。任何入口不得以本地推断覆盖 canonical status。

<a id="des-wr-ia-009"></a>
### DES-WR-IA-009：稳定追踪与证据绑定

长期设计只保存 `REQ/AC -> PRD-GAME-014 -> DES-WR-IA-* -> 验证方法`。每次实现由实际 `Task UID` 在 GitHub issue evidence 中绑定 source/integration/tested tree、PR、CI、评审和产物；任务状态、动态 HEAD 和 verdict 不回写本文。`TASK-GAME-071~075` 是玩法工作包标签，不替代 GitHub Task UID。

<a id="des-wr-ia-010"></a>
### DES-WR-IA-010：当前授权范围与高后果行动

对 [Agent authority PRD AC-2](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-2) / [AC-9](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-9)，runtime 按当前 owner/组织 authority 检查主体、对象/行动范围、有效期和剩余累计额度；拆分、并发、重试、重连或控制权切换不得复制或重置额度。Agent 提议不授予权限；权限、额度或来源无法证明时不得产生高后果世界效果。额度及经济规则由对应 gameplay/runtime authority 拥有，本文不定义其 schema 或公式。

<a id="des-wr-ia-011"></a>
### DES-WR-IA-011：授权变化与转让边界上的待决行动

对 [Agent authority PRD AC-8](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-8) / [AC-4](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-4)，撤销/到期、转让达到专业定义的生效点或新硬边界出现后，未产生权威效果的请求必须按当前 authority 重评；依现有专业合同继续、明确拒绝/取消/过期，或要求新的有效确认。不得静默沿用旧授权、自动换主体重放或追溯改写已提交结果。Agent 身份与历史保持可关联；转让条件及生效时间由 gameplay/ownership authority 决定。

<a id="des-wr-ia-012"></a>
### DES-WR-IA-012：异议、owner override 与因果归因

对 [Agent authority PRD AC-5](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-5) / [AC-6](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-6)，权威结果与 receipt 保留适用的 Agent 建议/执行、owner/组织决定、硬阻断和执行载体之间的关联。异议不构成世界拒绝；有效 owner override 仅在当前授权和世界规则范围内生效，不能越过 runtime 或安全硬边界。复用 DES-WR-IA-007 / 008 的提交、隐私和投影边界，不新增 receipt 字段或暴露私有 prompt。

## 5. 关键运行流程

### 5.1 接受、执行与结果

1. 玩家通过受支持入口提交带调用者和请求身份的意图。
2. 入口返回 request/validation outcome；尚无权威 receipt 时不得显示世界已应用。
3. Agent 可以产生解释或计划；runtime 依据当前世界、权限和规则接受、拒绝或阻塞。
4. runtime 在提交点产生权威状态、世界差异和可关联 receipt。
5. Viewer/API 投影相同的主意图、状态、主因果与下一步；无 progress 时明确表达。

### 5.2 阻塞、重排与恢复

1. 权威状态报告 blocker 或 bounded window 内无可验证推进。
2. projection 提供 wait/repair/reroute/reprioritize 或返回目标选择的适用集合及 tradeoff。
3. 玩家提交 interrupt/correction/new intent；系统保留旧新身份和 handoff reason。
4. runtime 决定何时生效；失败时保留原状态并返回可执行下一步。
5. 重连从 snapshot/journal/receipt 恢复同一因果链；身份不匹配或证据缺失时降级为重新决策，不伪造 continuation。

### 5.3 重试、并发与取消

重复请求必须依据明确 identity/idempotency 规则去重或成为新意图；当前 runtime 使用确定性 intent identity 与 request digest，未来变更必须保持冲突 fail closed。超时只证明当前观察窗口无结果。并发修改由 authority 决定顺序、拒绝或 supersede；取消只有收到权威结果后才可呈现为已停止。

### 5.4 Agent authority 变化与高后果结果

runtime 以当前授权来源复核高后果请求及适用累计额度，再独立执行世界/安全校验；检查失败时不产生对应世界效果。授权或硬边界变化影响待决请求时，所有尚未产生权威世界效果的请求都按变化后的当前授权与前置条件重新评估，包括已提交或已接受的请求；提交/接受状态、客户端排队或生命周期事件本身不证明世界效果已生效。按现有合同继续、拒绝、过期、取消或要求新的有效确认；已有权威世界结果不得被改写为未发生。权威结果关联适用的建议、有效决定/override、阻断原因与恢复下一步；重连和重试不能代替新决定或产生第二次效果。

## 6. 接口与数据合同

| 接口/条款 | producer -> consumer | identity/version | ordering/idempotency | success/error | compatibility |
| --- | --- | --- | --- | --- | --- |
| intent submission / DES-WR-IA-001 | Viewer/API -> Agent/runtime | caller + request/intent identity；具体 schema 由专业 authority 定义 | 重试规则必须显式 | accepted/rejected/pending 与 applied 分离 | 新字段不得改变旧状态含义 |
| interpretation / DES-WR-IA-001, DES-WR-IA-007 | Agent -> runtime | 关联 intent 与 Agent contract version | advisory；不得自行提交世界 | plan/reason 或结构化失败 | 旧 Agent 输出不能绕过 validation |
| authoritative result / DES-WR-IA-002 | runtime -> journal/receipt | world/candidate + intent/action identity | canonical order；可重放 | canonical `committed/rejected/failed/pending`；blocker 由 reason/disposition 表达 | gameplay 分类保留原 disposition，不新增 runtime enum |
| agency projection / DES-WR-IA-005, DES-WR-IA-008 | `WorldSnapshot.player_gameplay` / receipt -> Viewer/API | 同候选、调用者、时间/版本边界 | 只读投影；local feedback 仅 provisional overlay | 四类 invariant 或结构化 unavailable | 两入口语义等价，不要求 payload 相同 |
| resume / DES-WR-IA-006 | snapshot/journal/receipt -> continuation surface | world/session + continuation identity | 恢复必须幂等或显式冲突 | resumed/stale/replan-required | 旧快照缺字段时不得伪造当前值 |
| authorization / DES-WR-IA-010 | owner/organization authority -> runtime decision | 专业 authority 定义的主体、范围、对象与有效期 | 权威判定时复核；不由 Agent 或 WASM grant 推导 | 失效/超限/未知时无效果 | 不新增凭据或授权 schema |
| pending-action recheck / DES-WR-IA-011 | authority change -> any pending intent without an authoritative world effect | 既有 request/intent identity + 生效边界；submitted/accepted 状态本身不证明世界效果已生效 | 每个尚未产生权威世界效果的请求都重评，包括已提交或已接受请求；已有权威世界结果不得被改写为未发生 | 按现有合同继续、拒绝、过期、取消或新确认 | 转让合同留在 gameplay/ownership authority |
| attribution / DES-WR-IA-012 | Agent reason + owner/organization decision + runtime result -> receipt/projection | 既有意图、authority 来源与权威结果 | 与结果保持同一因果关联 | 区分异议、override、硬阻断和执行结果 | 遵守 DES-WR-IA-007/008 隐私边界 |

具体字段集合继续由 [`PRD-GAME-014`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#2-user-experience-functionality) 与各专业 authority 管理；本文只冻结跨组件语义和禁止条件。

## 7. 状态、事务与持久化

- 状态身份：玩法分类必须保留对应 authoritative disposition 与意图身份；同名状态不能脱离版本和 producer 被拼接。
- 提交点：世界效果只有在 runtime authority 的提交/receipt 边界后成立；UI ack、Agent plan 与 queue admission 不是提交点。
- 原子性：意图交接至少保证旧意图、替换原因和新意图结果可关联；若跨组件不能原子提交，必须暴露 pending/partial 并支持恢复。
- 重放与幂等：replay 必须得到同一权威次序与结果，或明确报告不兼容/冲突；投影层不得在重放时生成新的世界效果。
- 持久化：snapshot、journal 与 receipt 的 owner、保留和恢复由 runtime authority 定义；Viewer cache 和 Agent working state 不能代替它们。
- 生命周期用词：`accepted`、`applied`、`persisted`、`published` 分别需要接受结果、世界提交、持久化确认和可消费发布边界，任一缺失都保持 pending/unknown。
- 授权有效性和意图/结果生命周期分开；authority 变化触发未提交行动复核，已提交结果与归因历史保持可审计且不可追溯重写。

## 8. 部署、安全与运行约束

- 玩家只能读取和修改自己有权控制的意图面；组织/治理/冲突动作继续受各自 authority 校验。
- Viewer/API 不持有 world write authority；Agent 不绕过 runtime 权限与世界规则。
- owner/组织只能在当前有效范围内授权或 override Agent；WASM module grant、provider capability、Viewer session 与 Agent 自述均不等同此授权，也不能绕过硬边界。
- 记忆与理由只输出完成当前决策所需的脱敏摘要；私有 prompt、内部 trace、其他玩家记忆和凭据不得进入 receipt。
- provider、网络或入口不可用时，系统必须报告 unavailable/pending/blocked 的真实边界；deterministic/debug lane 不能冒充 active-LLM 或真实集成证据。
- 超时、资源预算和最大 payload 由具体接口 authority 定义。缺少对应证据时，本设计只要求结构化失败与可恢复下一步，不承诺性能。

## 9. 质量与容量

| 场景 | 环境、规模与资源 | 预期响应 | 指标/阈值来源 | 验证入口 | 当前范围 |
| --- | --- | --- | --- | --- | --- |
| accepted vs applied | 受支持意图、固定候选 | 任一入口不把 ack 当作 world effect | [`PRD-GAME-014` acceptance](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#prd-game-014-acceptance) | contract + integration test | 具体入口 coverage 未验证 |
| bounded response | PRD 定义的观察窗口 | 无推进时升级为 blocker/fallback/replan | [`NFR-CFC-6`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#prd-game-014-nfr) | QA matrix / session evidence | 时间阈值由专业 authority 冻结 |
| cross-entry parity | 同候选 Viewer 与 pure API | 四类 invariant 语义一致 | [`NFR-CFC-2`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#prd-game-014-nfr) | paired API/Web comparison | payload 可不同 |
| resume/replay | 可恢复 snapshot/journal/receipt | 恢复同一意图链或明确 replan-required | AC-WR-IA-002 | replay/reconnect scenario | 跨版本兼容需候选证明 |
| memory correction | 实际使用长期记忆的决策 | 摘要、来源、纠正与结果可关联 | [`NFR-CFC-7`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#prd-game-014-nfr) | Agent + surface scenario | 不要求未使用记忆的流程引入记忆 |

确定性、长期运行、真实 provider、浏览器和跨入口组合能力都必须以对应环境证据判断；文档检查不证明这些能力。

## 10. 兼容、迁移与回滚

- 新设计条款先建立映射，不自动改变现有 runtime enum、schema 或 transport。
- 实现若新增字段，应保持旧消费者能够读到明确 unavailable/unknown，而不是推导伪值；删除/重命名必须提供版本迁移与 consumer 验证。
- 旧快照或 journal 缺少 agency 字段时，恢复面应回退为 replan-required 或有界的 legacy summary，不能声称完整 continuation。
- rollout 按意图类型和正式入口逐项绑定 task 与证据；一个入口通过不能推导全部入口通过。
- 回滚回到已知实现基线时，必须同时撤回不再成立的 capability claim；已经产生的权威世界效果不可由 UI/文档回滚。

## 11. 验证设计与可追溯性

稳定链路为：

```text
产品 REQ/AC
  -> PRD-GAME-014 guarantee / failure signature
  -> DES-WR-IA-*（本文）
  -> GitHub Task UID + Issue evidence
  -> PR / fixed source / integration / tested tree
  -> test、role review、CI、artifact 与交付结论
```

### 11.1 验证映射表

| 上游 requirement / acceptance | 本设计条款 | 独立 obligation | 验证方法与 layer | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [`REQ-WR-IA-001`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-001) | [DES-WR-IA-001](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-001) | ack 不冒充 applied；world change 与 intent 可关联 | [`snapshot_progress.rs`](../../../crates/oasis7/src/viewer/runtime_live/tests/snapshot_progress.rs) 的 `compat_snapshot_surfaces_control_feeling_contract_fields_from_gameplay_feedback`；required runtime layer，使用 task 冻结候选 | Task issue evidence + CI/artifact | 未纳入的意图/入口 |
| [`REQ-WR-IA-002`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-002) | [DES-WR-IA-003](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-003) | queued intent 的 wait-resolution quote 提供可观察复查触发、预期变化与 alternative unlock condition，且不把 `safe_to_wait` 由快照单独推导为 true | [`wait_resolution_quote.rs`](../../../crates/oasis7/src/viewer/runtime_live/tests/wait_resolution_quote.rs) 的 `compat_snapshot_quotes_when_a_queued_intent_can_be_safely_waited_on`；targeted snapshot-projection layer，使用 task 冻结候选 | Task issue evidence + QA matrix | blocker recovery、handoff/return causal preservation、跨版本/跨设备范围 |
| [`REQ-WR-IA-002`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-002) | [DES-WR-IA-006](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-006) | 离开/重连从 continuation evidence 恢复，失效时进入 replan-required | [`auth_actions_provider_continuation_restart.rs`](../../../crates/oasis7/src/viewer/runtime_live/tests/auth_actions_provider_continuation_restart.rs) 的 `runtime_provider_wait_real_checkpoint_blocker_retries_same_server_then_reload`；required + full recovery layer，使用 task 冻结候选 | Task issue evidence + QA matrix | 跨版本/跨设备范围 |
| [`REQ-WR-IA-003`](../../product/world-rules-core-gameplay/indirect-control-agency-and-continuation.prd.md#req-wr-ia-003) | [DES-WR-IA-007](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-007) | reason/stakes/alternative/correction/result 同链 | [`agent_cognition_live_harness.rs`](../../../crates/oasis7/src/simulator/tests/agent_cognition_live_harness.rs) 的 `target_actor_memory_intents_require_matching_committed_runtime_receipt_exactly_once`；required + full layer，使用 task 冻结候选 | Task issue evidence + review artifact | 完整 stakes/alternative/correction receipt 当前未证明 |
| [`PRD-GAME-014 causal-decision receipt`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#21-reusable-causal-decision-receipt) | [DES-WR-IA-009](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-009) | gameplay receipt 与跨入口证据绑定同一候选 | [`testing-manual.md`](../../../testing-manual.md) 的 S6 Web UI 闭环 smoke；headed browser + pure API full layer，使用 task 冻结候选 | Task issue evidence + browser/CI artifact | 真实同候选 parity 当前未证明 |
| [`PRD-GAME-014 causal-decision receipt`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#21-reusable-causal-decision-receipt) | [DES-WR-IA-007](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-007) | memory context、correction 与 committed runtime receipt 可关联 | [`agent_cognition_live_harness.rs`](../../../crates/oasis7/src/simulator/tests/agent_cognition_live_harness.rs) 的 `target_actor_memory_intents_require_matching_committed_runtime_receipt_exactly_once`；required + full layer，使用 task 冻结候选 | Task issue evidence + review artifact | 完整 stakes/alternative/correction receipt 当前未证明 |
| [`PRD-GAME-014 causal-decision receipt`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md#21-reusable-causal-decision-receipt) | [DES-WR-IA-008](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-008) | Viewer/API 同源输出 accepted、status、reason、next step 且不越权 | [`testing-manual.md`](../../../testing-manual.md) 的 S6 Web UI 闭环 smoke；headed browser + pure API full layer，使用 task 冻结候选 | Task issue evidence + browser/CI artifact | 真实同候选 parity 当前未证明 |
| [`AC-1`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-1) | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | 高自治/有界授权不改变世界、资源、权限与治理边界 | [`agent_intent_v2.rs`](../../../crates/oasis7/src/runtime/tests/agent_intent_v2.rs) `accepted_intent_v2_roundtrips_authority_position_and_freshness` is an authority-context baseline; delivery task adds the no-boundary-expansion scenario; deterministic runtime, `test_tier_required` | Delivery task Issue + CI artifact | Existing intent test does not establish autonomy or owner-grant boundaries |
| [`AC-2`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-2) | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | 有效授权、范围/到期/撤销及硬阻断负例都不产生越权世界效果 | [`agent_cognition_live_command.rs`](../../../crates/oasis7/src/runtime/tests/agent_cognition_live_command.rs) `governed_typed_command_binds_turn_context_quote_and_live_recheck` is a capability-grant recheck analogue; delivery task adds owner delegation cases; deterministic runtime integration, `test_tier_required` | Delivery task Issue + CI artifact | Module-grant coverage does not prove player/organization delegation |
| [`AC-3`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-3) | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | 团队扩张不静默复制授权/责任，同时消费 gameplay 的团队资产条件 | [`agent_claims.rs`](../../../crates/oasis7/src/runtime/tests/agent_claims.rs) `reputation_tier_scales_second_slot_and_enforces_claim_cap` is an Agent slot/economy baseline; delivery task adds the authority-and-asset acceptance scenario; deterministic runtime, `test_tier_required` | Delivery task Issue + CI artifact | Existing cap test does not cover authority copying or responsibility |
| [`AC-4`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-4) | [DES-WR-IA-011](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-011) | 转让生效后新授权生效；身份、来源及历史不断链 | [`agent_intent_v2.rs`](../../../crates/oasis7/src/runtime/tests/agent_intent_v2.rs) `replaced_intent_event_is_replayable_without_promoting_activity_or_receipt` is an intent-history baseline; delivery task adds owner-transfer continuity; deterministic transfer/runtime, `test_tier_required` | Delivery task Issue + CI artifact | Existing replay test does not cover ownership transfer; transfer economy remains with gameplay |
| [`AC-5`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-5) | [DES-WR-IA-012](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-012) | 区分 Agent 异议、有效 owner override 与 world/safety hard block | [`agent_intent_v2.rs`](../../../crates/oasis7/src/runtime/tests/agent_intent_v2.rs) `blocked_intent_cannot_resume_without_a_dedicated_authority_proof` is a hard-block baseline; delivery task adds objection and valid-override cases; runtime receipt, `test_tier_required` | Delivery task Issue + CI artifact | Existing test does not distinguish Agent objection or owner override |
| [`AC-6`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-6) | [DES-WR-IA-012](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-012) | 责任归因覆盖适用 Agent/owner/组织/执行载体且不把 Agent 当责任替身 | [`agent_cognition_live_harness.rs`](../../../crates/oasis7/src/simulator/tests/agent_cognition_live_harness.rs) `target_actor_memory_intents_require_matching_committed_runtime_receipt_exactly_once` is a receipt-lineage baseline; delivery task adds actor attribution and authorized projection; runtime receipt/projection, `test_tier_full` | Delivery task Issue + CI/review artifact | Existing receipt test does not prove the full attribution chain |
| [`AC-7`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-7) | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | 工业供给或 quota 不静默扩大权限；组合中仍验证专业授权来源 | [`gameplay_protocol_regressions.rs`](../../../crates/oasis7/src/runtime/tests/gameplay_protocol_regressions.rs) `economic_contract_respects_policy_quota_and_block_list` is an economy-quota baseline; delivery task adds Agent-authority interaction; deterministic runtime, `test_tier_full` | Delivery task Issue + CI artifact | Existing economic test does not establish Agent authority; industrial economics stay with gameplay |
| [`AC-8`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-8) | [DES-WR-IA-011](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-011) | 撤销/到期/转让/硬边界后的待决请求重评或结束，已提交历史不变 | [`agent_intent_v2.rs`](../../../crates/oasis7/src/runtime/tests/agent_intent_v2.rs) `live_terminal_apis_persist_expiry_and_cancellation_for_exact_identity` is an expiry/cancellation baseline; delivery task adds revoke/transfer recovery; deterministic runtime recovery/replay, `test_tier_full` | Delivery task Issue + CI artifact | Existing test does not cover delegation revocation or transfer; durable recovery integration remains unverified |
| [`AC-9`](../../product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md#ac-9) | [DES-WR-IA-010](indirect-control-agency-execution-and-continuation.design.md#des-wr-ia-010) | 同一授权额度累计检查，拆分/并发/重试/重连/主体切换不复制、不重置 | [`agent_cognition_live_command.rs`](../../../crates/oasis7/src/runtime/tests/agent_cognition_live_command.rs) `governed_typed_command_no_effect_is_free_and_effect_is_debited_once_on_retry` is a retry-accounting baseline; delivery task adds cumulative authority/quota and concurrency; deterministic runtime concurrency/replay, `test_tier_full` | Delivery task Issue + CI artifact | Existing retry test does not establish cumulative authority quotas |

### 11.2 本次交付连接

本文首次系统化重建由 GitHub Task UID `task_8b5664cfc5b345758e0ed17966fd57c5` 与 Issue `#3712` 追踪。该引用只说明设计发布 provenance；任务的当前状态、HEAD、PR、CI 和评审结果必须从 issue evidence 读取，不构成本文的长期状态字段。

## 12. 决策、长期风险与未决问题

| 决策 ID | 选择 | 未选择 | 理由与失效触发 |
| --- | --- | --- | --- |
| ADR-GAME-IA-001 | 以统一 causal chain 串联多组件 | 各入口自行组装控制叙事 | 防止 accepted/applied 和主因果漂移；authority 模型改变时复核 |
| ADR-GAME-IA-002 | 设计条款使用稳定 `DES-WR-IA-*` | 直接用 Task/TASK-GAME 标签充当设计身份 | 长期设计与一次交付生命周期分离；ID 规范变更时迁移并留映射 |
| ADR-GAME-IA-003 | parity 定义为语义 invariant 等价 | 强制 UI/API payload 相同 | 保留入口实现自由；任一入口无法证明四类 invariant 时失效 |
| ADR-GAME-IA-004 | gameplay 分类映射 runtime disposition | 在本文新增 runtime enum | 尊重 runtime authority；现有 disposition 无法表达义务时另开实现 task |

长期风险与未决问题：

- 哪些意图类型、入口、版本和环境进入第一批可验证范围，由 `producer_system_designer` 联合各专业 owner 在实际 task 中冻结；未绑定者保持 unsupported/unknown。
- 具体 intent/idempotency/continuation identity、bounded-response 时间窗口和 receipt schema 由对应专业 authority 设计并版本化；在此之前本文不声称已实现。
- Gameplay PRD 的 acceptance 与 NFR 已补稳定 HTML anchor；如果后续出现逐条机器消费需求，再由 gameplay owner 为 AC/NFR 单项补充稳定 anchor 与迁移映射。
- 最大风险是只补 UI 文案或只恢复文档，却没有 canonical truth 与同候选证据；这种状态必须继续报告为设计已定义、实现未验证。当前测试只覆盖 canonical intent、override/reprioritize、wait quote、continuation 和 memory receipt 的若干切面；完整 causal receipt 与同候选 Viewer/pure API parity 仍未证明。
