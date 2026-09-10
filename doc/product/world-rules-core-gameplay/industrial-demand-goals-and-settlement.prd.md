# 工业需求目标与生产交付结算

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 配对产品 design：[`industrial-demand-goals-and-settlement.design.md`](industrial-demand-goals-and-settlement.design.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/game/gameplay/gameplay-top-level-design.prd.md`](../../game/gameplay/gameplay-top-level-design.prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)
- 适用入口：Viewer、pure API、Agent 代办所共享的工业需求目标读面

本文把需求目标变成玩家可以比较、提交、复盘和继续规划的工业循环。它承载玩家可观察的目标、数量层次、机会成本、结算边界和恢复选择；正式配方、批量、产率、价格、物流、电力、终端容量、receipt、runtime schema、队列和 UI 由上列专业权威负责。首局工业 walkthrough 仍由[`首局与持续游玩`](first-session-and-continuation.prd.md)负责；本专题定义可在首局之后反复使用的 demand-goal 语义，不重复首局引导。

## 1. 玩家问题与目标

### 1.1 代表性情境

玩家已经有一个工业目标，例如为某个受支持的终端用途交付一批产品。提交前，玩家需要知道目标还缺多少、已有多少会在路上、哪些投入和容量会被占用，以及把整批生产、减量生产、补料/调运、暂存盈余或暂缓目标分别会带来什么代价。生产完成后，玩家还需要知道产品是否真正匹配了目标并完成了交付结算；不足时，玩家必须能选择补产或保留缺口，而不是被后台循环替玩家改写目标。

### 1.2 目标与成功含义

本专题解决“我正在为哪个目标投入什么、目标为什么还没有完成、下一步有哪些真实选择”的问题。成功的玩家体验是：玩家从同一份当前权威事实作出一次可解释承诺；世界分别记录承诺、生产、交付和终端结算；每次结果打开下一次有边界的选择，且失败、盈余、重连和重复操作不制造免费进度或隐藏损失。

### 1.3 依据与假设

- 已采纳产品承诺：根 PRD 的 SC-31 要求比较需求、数量分层、批量与资源占用，区分生产和匹配结算，处理盈余、缺口、漂移、重复执行并保持多入口同义。
- 专业约束：[`PRD-GAME-012`](../../game/prd.md) / [`PRD-GAME-014`](../../game/prd.md) 负责玩家决策和间接控制语义；[`PRD-WORLD_RUNTIME-019`](../../world-runtime/prd.md) / [`PRD-WORLD_RUNTIME-043`](../../world-runtime/prd.md) 负责工业状态、root/revision、receipt、持久化和幂等目标；[`PRD-WORLD_SIMULATOR-047`](../../world-simulator/prd.md) 负责批次、路径、终端和守恒合同；[`PRD-TESTING-003`](../../testing/prd.md) 负责组合证据。
- 设计假设：把“生产中”“已生产”“已匹配交付”和“终端已结算”分开，会让玩家能把投入与目的地结果归因到具体选择。该假设需要适用入口的玩法证据验证；本专题采纳不等于实现或可玩性结论。

### 1.4 未决问题与决策条件

- 产品级未决问题：无未决的工业需求目标行为义务；本专题的 REQ/AC 已表达当前产品边界。
- 仍需专业 authority 决定的条件：正式配方、批量、产率、价格、物流、电力、容量、receipt、runtime schema、队列和入口支持范围，在具体能力进入当前 claim 或实现输入前必须由文档身份中列出的 authority 与 fresh 证据确认；缺失、冲突或过期时保持 `unknown/blocked`。
- 接收 owner 与触发条件：对应 gameplay、runtime、world-simulator、Viewer 和 QA owner 在该能力进入本专题承诺范围或发生专业合同变化时更新 task evidence；产品正文保留语义回链，不复制执行台账。

## 2. 范围与 Non-Goals

范围包括：一个有权威身份的 demand goal；提交前的目标/数量/资源/容量比较；专业合同支持的 full、reduced、补料/调运、持有盈余、停止/延期和 parent-linked 补产选择；production receipt、matching delivery 或 terminal settlement 的玩家含义；目标满足、缺口、盈余、漂移和重复执行后的恢复。

不包括：首局教学节奏、正式批量或产率公式、订单簿和价格机制、自动补货/自动停机、队列算法、runtime/API/schema、Agent 实现、Viewer 布局、测试脚本或当前版本完成声明。不存在专业合同支持的选项时，产品层不能把它写成玩家可用动作。

## 3. 玩家体验与决策

### 3.1 正常路径

| 阶段 | 玩家知道什么 | 可以选择什么 | 主要代价或承诺 | 可观察结果 | 下一步 |
| --- | --- | --- | --- | --- | --- |
| 目标报价 | `goal_authority_ref`、目标量、已承诺/生产/交付/终端结算/剩余数量，以及 batch quantum、输入、电力、物流、buffer、terminal 占用和下一复查点 | 比较专业合同支持的 full/reduced、补料/调运、hold、stop/defer | 把当前权威快照中的资源、容量、机会成本转成可追溯选择；报价本身不产生效果 | 一份可比较的 demand-goal preview；未知事实保持 `unknown/blocked` | 确认、重新获取报价或回到目标选择 |
| 提交承诺 | 玩家知道选择绑定的目标、数量和当前条件 | 确认一个支持的计划，或放弃/改选 | 接受可能形成有限承诺；提交必须按新鲜权威状态重验 | `accepted`、`blocked`、`requote` 或专业合同支持的有界 pending；不能把 accepted 当作生产或交付 | 等待、修复、补料、调运、重排或停止 |
| 生产与运输 | 已承诺、已生产、在途、buffer-held 的数量分别代表什么 | 按专业合同支持的 wait、hold、repair、reroute 或 stop | 已发生的 sink、损耗和仍占用容量必须保持可读 | production receipt 只说明实际生产结果；不减少 demand，不发 delivery reward | 等待 matching settlement、处理 blocker 或作一次补产决策 |
| 匹配结算 | 哪一份 delivery/terminal settlement 与目标匹配，已满足多少，还缺多少 | 接受结果、检查非 matching settlement、选择补产或停止/延期 | 结算只按匹配数量减少目标；非匹配结果不提供目标进度 | 目标满足、partial shortage、surplus 或 blocked 的明确状态 | 继续另一个目标、一次 parent-linked 补产或保留缺口 |

### 3.2 主要失败与恢复

- 目标、库存、batch quantum、产率或 terminal capacity 发生漂移时，旧报价不能继续代表可用承诺。玩家看到重新报价、无副作用原子拒绝或专业合同明确支持的有界 pending，并能知道需要补料、调运、等待、改目标或停止。
- matching settlement 不足时，玩家只能在专业合同支持的范围内选择一次 parent-linked supplemental revision，或保留 shortage 并停止/延期。停止/延期保留缺口，不生成补产 revision。
- 合法批量超过目标时，玩家看到剩余目标与 surplus 的分层。系统不得替玩家倾销、销毁、伪造成交或把 surplus 计为成长；后续处置必须是专业合同支持的显式选择。
- 没有安全路径时，界面和 Agent 读面应明确停止以及下一次复查或目标选择点，不以无限等待、后台补产或“已完成”标签掩盖失败。

### 3.3 设计说明

该循环把“生产有进展”与“需求已满足”分开，保留玩家在容量、投入、时间和目标之间的真实取舍。玩家可以追求整批的效率，也可以选择减量、补料、持有盈余或暂缓；每条路径都必须有专业合同支持和可读机会成本。这样既保留工业经营的决策价值，也避免把批量四舍五入、后台 Agent 行为或运输到达误报为需求完成。

<a id="sc31-leaf-requirements"></a>
## 4. SC-31 叶子要求

根 SC-31 保留完整组合承诺和既有六列追踪。本节只把其可独立判定的义务下钻为稳定局部 ID；局部 ID 不替代根 SC，也不改变专业域权威。

### 4.1 原义务到目的地映射

| 原 SC-31 义务（原文保留） | 目的地叶子要求 | 验收场景 |
| --- | --- | --- |
| 提交前从同一权威快照比较目标量、已承诺/生产/交付量、batch quantum、缺口/匹配/盈余、输入/电力/物流/buffer/terminal 占用、机会成本和下一复查点 | REQ-SC31-001 | AC-SC31-001 |
| 只展示专业合同真实支持的 full/reduced、补料/调运、持有盈余、停止/延期或 parent-linked 补产路径 | REQ-SC31-002 | AC-SC31-002 |
| production receipt 不减少需求或发交付奖励；只有 matching delivery/terminal settlement 更新目标满足量；非 matching settlement 不重复减少需求 | REQ-SC31-003 | AC-SC31-003 |
| 目标满足后停止旧 schedule、后台循环或 Agent retry；合法批量盈余不得自动倾销、销毁、伪成交或计成长 | REQ-SC31-004 | AC-SC31-004、AC-SC31-005 |
| matching settlement 不足时，一次可追溯 supplemental revision 保留 baseline、actual、损耗、已满足量和缺口；停止/延期保留 shortage 且不创建 revision | REQ-SC31-005 | AC-SC31-006 |
| 目标、库存、批量、产率或 terminal capacity 漂移触发重报价、无副作用原子拒绝或 profile 明示有界 pending | REQ-SC31-006 | AC-SC31-007 |
| 重复 submit/delivery、重连、乱序、恢复和 replay 不复制生产、交付、需求减少、盈余处置、奖励或容量释放 | REQ-SC31-007 | AC-SC31-008 |
| Viewer、pure API 与 Agent 对 demand 状态、数量分层、blocker、动作和复查点保持同义 | REQ-SC31-008 | AC-SC31-009 |

### 4.2 叶子要求

<a id="req-sc31-001"></a>
#### REQ-SC31-001：同一权威快照的需求比较

- 性质：已采纳目标
- 适用条件：玩家在 demand goal 提交前比较一个有 `goal_authority_ref` 的目标。
- 要求：产品读面必须（MUST）从同一份当前权威快照分别显示 target、committed、produced、delivery-settled、terminal-settled、remaining 数量，canonical batch quantum，预计 shortage/matched/surplus，输入/电力/物流/buffer/terminal 占用，机会成本和 `next_recheck`；任何缺失、过期或冲突 authority 必须显示为 `unknown/blocked`，不得补成安全或零成本。
- 理由：玩家需要知道一次计划消耗了什么、已推进到哪里以及还有什么选择。
- 上位承诺：SC-31。
- 专业权威：[PRD-GAME-012](../../game/prd.md)、[PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)。
- 验收：AC-SC31-001。

<a id="req-sc31-002"></a>
#### REQ-SC31-002：受支持的计划选择

- 性质：已采纳目标
- 适用条件：目标报价存在可继续、受限或不可用的候选路径。
- 要求：产品读面必须（MUST）只呈现专业合同真实支持的 full、reduced、补料/调运、hold surplus、stop/defer 或 parent-linked supplemental 路径，并为每个可选路径说明作用范围、追加成本或仍占用价值、预计结果/复查点、主要风险和 opportunity cost；preview 与 recommendation 不得创建世界效果。
- 理由：不同批量和恢复路径应当是有代价的玩家决策，而不是后台自动选择。
- 上位承诺：SC-31。
- 专业权威：[`gameplay` 需求目标计划规则](../../game/gameplay/gameplay-top-level-design.prd.md#25-前期工业引导成就闭环)；[PRD-GAME-012](../../game/prd.md)、[PRD-GAME-014](../../game/prd.md)（玩家决策与间接控制支持）；[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)。
- 验收：AC-SC31-002。

<a id="req-sc31-003"></a>
#### REQ-SC31-003：生产与匹配结算分层

- 性质：已采纳目标
- 适用条件：目标链路产生 production receipt、delivery receipt 或 terminal settlement。
- 要求：产品读面必须（MUST）把 production receipt 表达为生产结果，而只让 profile 声明的 matching delivery 或 terminal settlement receipt 更新目标满足量和交付奖励；非 matching settlement 不得减少目标，亦不得二次减少同一需求。
- 理由：玩家需要知道“做出来了”与“交到正确目的地并结算了”是两个不同结果。
- 上位承诺：SC-31。
- 专业权威：[PRD-GAME-014](../../game/prd.md)、[PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)、[PRD-WORLD_RUNTIME-043](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)。
- 验收：AC-SC31-003。

<a id="req-sc31-004"></a>
#### REQ-SC31-004：满足后停止与盈余保护

- 性质：已采纳目标
- 适用条件：目标已由匹配结算满足，或合法 batch 产出超过 remaining。
- 要求：产品读面必须（MUST）在目标满足后把旧 schedule、后台循环和 Agent retry 解释为停止追加该目标；对合法 batch 产生的 surplus，系统不得自动倾销、销毁、伪造成交或计为成长，玩家只能看到其真实状态和专业合同支持的后续处置。
- 理由：完成目标应关闭该目标的追加生产动机，同时保留批量造成的真实机会成本和资产结果。
- 上位承诺：SC-31。
- 专业权威：[PRD-GAME-012](../../game/prd.md)、[PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)。
- 验收：AC-SC31-004、AC-SC31-005。

<a id="req-sc31-005"></a>
#### REQ-SC31-005：缺口与补产修订

- 性质：已采纳目标
- 适用条件：matching settlement 只满足部分目标，或玩家主动选择停止/延期。
- 要求：产品读面必须（MUST）允许玩家在专业合同支持时选择一次 parent-linked supplemental revision，并保留原 baseline、actual、实际损耗、已满足量和 remaining shortage；若玩家选择 stop/defer，必须保留 shortage 且不得创建 supplemental revision。
- 理由：补产应是明确的新因果选择，停止或延期应保留真实缺口而不是伪造完成。
- 上位承诺：SC-31。
- 专业权威：[PRD-GAME-014](../../game/prd.md)、[PRD-WORLD_RUNTIME-043](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)。
- 验收：AC-SC31-006。

<a id="req-sc31-006"></a>
#### REQ-SC31-006：提交漂移的安全处置

- 性质：已采纳目标
- 适用条件：报价或承诺后目标、库存、batch quantum、产率或 terminal capacity 发生变化。
- 要求：产品读面必须（MUST）把漂移导向当前条件重报价、无副作用原子拒绝或 profile 明示的有界 pending；不得静默沿用旧数量、旧容量、旧风险或旧目标承诺，也不得在拒绝时产生新的 sink、义务或奖励。
- 理由：玩家应能分辨“条件改变”与“计划失败”，并获得真实的重规划入口。
- 上位承诺：SC-31。
- 专业权威：[PRD-GAME-012](../../game/prd.md)、[PRD-WORLD_RUNTIME-001](../../world-runtime/prd.md)、[PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)。
- 验收：AC-SC31-007。

<a id="req-sc31-007"></a>
#### REQ-SC31-007：重复执行与恢复幂等

- 性质：已采纳目标
- 适用条件：同一 submit、delivery、恢复处置或其事件发生重复、乱序、重连、恢复或 replay。
- 要求：产品读面必须（MUST）保持同一 root/revision 的一次性世界结果：重复操作不得复制 production、delivery、需求减少、surplus 处置、奖励或容量释放；历史实际损耗和已占用价值必须继续可追溯。
- 理由：重试和回流是恢复手段，不应成为复制进度或奖励的套利手段。
- 上位承诺：SC-31。
- 专业权威：[PRD-WORLD_RUNTIME-043](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)、[PRD-TESTING-003](../../testing/prd.md)。
- 验收：AC-SC31-008。

<a id="req-sc31-008"></a>
#### REQ-SC31-008：多入口同义

- 性质：已采纳目标
- 适用条件：同一 demand goal 从 Viewer、pure API 或 Agent 读面查看、推荐或恢复。
- 要求：三类入口必须（MUST）对 demand 状态、target/committed/produced/delivery-settled/terminal-settled/remaining 分层、shortage/matched/surplus/unknown、primary blocker、允许动作和 `next_recheck` 保持同义；入口形式可以不同，但不得制造不同的目标满足、奖励或恢复真值。
- 理由：玩家切换入口或授权 Agent 代办时仍应拥有同一条世界因果链。
- 上位承诺：SC-31。
- 专业权威：[PRD-GAME-014](../../game/prd.md)、[PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)、[PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)、[PRD-TESTING-003](../../testing/prd.md)。
- 验收：AC-SC31-009。

## 5. 验收与证据

<a id="ac-sc31-001"></a>
### AC-SC31-001：同一快照的提交前比较

给定：一个有效 `goal_authority_ref`、目标量、已承诺/生产/交付/终端结算数量、canonical batch quantum，以及已知和未知的输入、电力、物流、buffer、terminal 事实。
当：玩家打开提交前 demand-goal preview。
则：读面按同一快照分别显示 target、committed、produced、delivery-settled、terminal-settled、remaining、shortage/matched/surplus、占用、机会成本和 `next_recheck`；任一过期或冲突 authority 显示 `unknown/blocked`，preview 不产生 sink、hold、queue、产出、结算或奖励。

覆盖：REQ-SC31-001。

<a id="ac-sc31-002"></a>
### AC-SC31-002：只展示真实支持的选择

给定：同一目标有 full、reduced、补料/调运、hold、stop/defer 中部分路径被专业 profile 支持，另一些不被支持。
当：玩家比较并选择一个候选。
则：读面只展示被支持的候选；每个候选分别说明成本、仍占用、风险、预计结果和复查点；未支持的路径不显示为可选，比较、推荐和放弃均无世界效果，确认后才进入 fresh revalidation。

覆盖：REQ-SC31-002。

<a id="ac-sc31-003"></a>
### AC-SC31-003：生产不等于需求满足

给定：一个目标已有 production receipt，但产物尚未取得匹配 delivery 或 terminal settlement；另有一份非 matching settlement。
当：玩家查看目标状态并重试读取 settlement。
则：production receipt 只增加 produced 层，不减少 remaining、不发 delivery reward；非 matching settlement 不减少目标；取得一份匹配 settlement 后只按其支持的数量减少一次目标，并保留 settled 与未交付层次。

覆盖：REQ-SC31-003。

<a id="ac-sc31-004"></a>
### AC-SC31-004：目标满足后的显式停止

给定：matching settlement 已将目标 remaining 变为零，而旧 schedule、后台循环或 Agent retry 仍可能再次提交同一目标。
当：系统处理旧计划或 retry。
则：旧计划不追加生产、不新增需求减少、不发重复奖励；玩家看到目标已满足和下一可决策点，继续生产必须是新目标或新的显式选择。

覆盖：REQ-SC31-004。

<a id="ac-sc31-005"></a>
### AC-SC31-005：合法 batch 的 surplus 不自动处置

给定：一个合法 batch 因 canonical quantum 产生超过 remaining 的产品。
当：batch 结算完成。
则：目标只按匹配数量满足，surplus 单独可见；系统不自动倾销、销毁、伪成交或把 surplus 计为成长，玩家只能选择专业合同支持的 hold 或后续处置。

覆盖：REQ-SC31-004。

<a id="ac-sc31-006"></a>
### AC-SC31-006：部分匹配后的补产或停止

给定：matching settlement 只满足部分目标，并保留原 baseline、actual、损耗、已满足量和 remaining shortage。
当：玩家分别选择一次 supplemental revision，或选择 stop/defer。
则：前者只创建一个与原目标 parent-linked 的可追溯补产 revision，不改写原 baseline/actual；后者保留 shortage 且不创建 revision；两种选择都不重复结算已满足量。

覆盖：REQ-SC31-005。

<a id="ac-sc31-007"></a>
### AC-SC31-007：条件漂移后的重新裁决

给定：报价后目标、库存、batch quantum、产率或 terminal capacity 至少一项发生变化。
当：玩家提交旧报价。
则：系统重新报价、无副作用原子拒绝，或进入 profile 明示的有界 pending；旧报价不创建新的 sink、义务、容量占用或奖励，玩家可读变化原因、保留结果和下一步。

覆盖：REQ-SC31-006。

<a id="ac-sc31-008"></a>
### AC-SC31-008：重复、乱序、恢复和 replay 只产生一次结果

给定：同一 submit、delivery 或恢复处置被重复投递、乱序到达、重连、snapshot restore 或 replay。
当：系统重新处理该事件或请求。
则：只返回或重建原 root/revision 结果；production、delivery、需求减少、surplus 处置、奖励和容量释放各至多发生一次，原有损耗、占用和 provenance 保持可读，不产生退款与完成的双重结果。

覆盖：REQ-SC31-007。

<a id="ac-sc31-009"></a>
### AC-SC31-009：Viewer、pure API 与 Agent 同义

给定：同一 demand goal、同一权威快照和同一 blocker。
当：玩家通过 Viewer 查看，或通过 pure API / Agent 请求相同状态、推荐或恢复路径。
则：三类入口对目标数量分层、shortage/matched/surplus/unknown、primary blocker、允许动作、机会成本和 `next_recheck` 给出同义结果；任何入口都不能把 production、buffer 或非 matching settlement 说成 demand 已满足或 reward 已发放。

覆盖：REQ-SC31-008。

### 5.1 追踪

| 叶子要求 | 上位承诺 | 专业权威 / PRD-ID | 验收场景 | 验证入口 |
| --- | --- | --- | --- | --- |
| REQ-SC31-001 | SC-31 | [`doc/game/prd.md`](../../game/prd.md) / [PRD-GAME-012](../../game/prd.md)；[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) / [PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)；[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) / [PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md) | AC-SC31-001 | `test_tier_required`：同一快照、数量分层、unknown/blocked、preview 无效果 |
| REQ-SC31-002 | SC-31 | [`gameplay` 需求目标计划规则](../../game/gameplay/gameplay-top-level-design.prd.md#25-前期工业引导成就闭环) / [PRD-GAME-012](../../game/prd.md)、[PRD-GAME-014](../../game/prd.md)（玩家决策与间接控制支持）；[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) / [PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md) | AC-SC31-002 | `test_tier_required`：候选支持边界与确认前无效果 |
| REQ-SC31-003 | SC-31 | [`doc/game/prd.md`](../../game/prd.md) / [PRD-GAME-014](../../game/prd.md)；[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) / [PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)、[PRD-WORLD_RUNTIME-043](../../world-runtime/prd.md)；[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) / [PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md) | AC-SC31-003 | `test_tier_required`：production/delivery/terminal settlement 分层与单次需求减少 |
| REQ-SC31-004 | SC-31 | [`doc/game/prd.md`](../../game/prd.md) / [PRD-GAME-012](../../game/prd.md)；[`world-runtime` 专业 PRD](../../world-runtime/prd.md) 与 [`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) | AC-SC31-004、AC-SC31-005 | `test_tier_required`：满足后停止、surplus 无自动处置 |
| REQ-SC31-005 | SC-31 | [`doc/game/prd.md`](../../game/prd.md) / [PRD-GAME-014](../../game/prd.md)；[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) / [PRD-WORLD_RUNTIME-043](../../world-runtime/prd.md)；[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) / [PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md) | AC-SC31-006 | `test_tier_required`；`test_tier_full` 覆盖 revision 链、部分匹配和持久化 |
| REQ-SC31-006 | SC-31 | [`doc/world-runtime/prd.md`](../../world-runtime/prd.md) / [PRD-WORLD_RUNTIME-001](../../world-runtime/prd.md)、[PRD-WORLD_RUNTIME-019](../../world-runtime/prd.md)；[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) / [PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md) | AC-SC31-007 | `test_tier_required`：drift 重报价/拒绝/pending |
| REQ-SC31-007 | SC-31 | [`doc/world-runtime/prd.md`](../../world-runtime/prd.md) / [PRD-WORLD_RUNTIME-043](../../world-runtime/prd.md)；[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md) / [PRD-WORLD_SIMULATOR-047](../../world-simulator/prd.md)；[`doc/testing/prd.md`](../../testing/prd.md) / [PRD-TESTING-003](../../testing/prd.md) | AC-SC31-008 | `test_tier_required`；`test_tier_full` 覆盖并发、跨窗口、恢复/replay/补偿 |
| REQ-SC31-008 | SC-31 | [`doc/game/prd.md`](../../game/prd.md) / [PRD-GAME-014](../../game/prd.md)；[`world-runtime` 专业 PRD](../../world-runtime/prd.md) 与 [`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md)；[`doc/testing/prd.md`](../../testing/prd.md) / [PRD-TESTING-003](../../testing/prd.md) | AC-SC31-009 | `test_tier_required`：Viewer/pure API/Agent parity |

### 5.2 产品效果与证据范围

该专题要证明的是玩家能把一次工业投入与目标进展、真实成本、失败边界和下一选择联系起来。`test_tier_required` 可证明合同可判定性、读面语义和一次性效果；`test_tier_full` 才覆盖并发目标、跨窗口到期/取消、三阶段多批次/损耗、容量争用、持久化/补偿以及多入口长期 parity。文档采纳、自动化通过或历史样例不能单独证明当前版本已经好玩、已经实现或适合公开发行；当前 claim 仍服从根 README、专业 evidence 和 `playability-evidence-and-claim-boundaries.prd.md`。

## 6. 权威与相邻专题

- `game` 拥有目标选择、机会成本、玩家动作和玩法平衡；`world-runtime` 拥有执行顺序、状态、持久化、root/revision、receipt 和 replay；M4 拥有批次、物流、终端、守恒和 profile 合同；QA 拥有组合验证。
- [`首局与持续游玩`](first-session-and-continuation.prd.md) 拥有首局工业 walkthrough 与首次持续能力；本专题只接收其已建立的目标/生产/交付边界，面向可重复的 ongoing demand goal。
- [`成熟世界成长与区域参与`](mature-world-progression.prd.md) 使用交付和区域需求作为长期方向；本专题不把库存或生产量直接计为成熟成长。
- [`可玩性证据与承诺边界`](playability-evidence-and-claim-boundaries.prd.md) 拥有玩家杠杆、继续理由和证据层级；本专题不自行升级产品 verdict。

## 7. 设计取舍与未决问题

### 7.1 取舍

采用“先比较、再承诺、分层结算、显式补产”的循环，是为了让玩家在 batch quantum、容量和交付用途之间作出可回顾的选择。把 surplus 与 shortage 保留为独立结果会牺牲自动化的表面顺滑，但能保住资源压力、目标因果和失败恢复，也能阻止重复提交变成奖励套利。

### 7.2 未决问题

- 尚未决定：不同 demand profile 对 full/reduced、partial matching、surplus hold 和 supplemental revision 的具体支持范围。影响：REQ-SC31-002、REQ-SC31-005。决策负责角色：`gameplay_designer` 联动 `producer_system_designer` 与对应专业 owner。需要 profile 合同和组合证据；在范围确定前，未声明路径保持不可选。任务引用：[`GitHub task #3650`](https://github.com/eng-cc/oasis7/issues/3650)，仅作为该决策的稳定 locator，不复制任务状态或宣称决策完成。
- 尚未决定：具体 batch quantum、产率、价格、capacity、expiry 和 queue policy。影响：REQ-SC31-001、REQ-SC31-006。决策负责角色：对应 `game` / `world-runtime` / M4 专业 owner。需要正式专业规则；本专题不临时给出默认值。任务引用：[`GitHub task #3650`](https://github.com/eng-cc/oasis7/issues/3650)，仅作为该决策的稳定 locator，不复制任务状态或宣称决策完成。
- 尚未决定：各正式玩家入口怎样承载同义 demand 读面。影响：REQ-SC31-008。决策负责角色：`viewer_engineer`、`agent_engineer`、`qa_engineer` 通过 TPM 协调；需要入口专属验证。文档层只要求语义 parity，不冻结布局、API 或 Agent prompt。任务引用：[`GitHub task #3650`](https://github.com/eng-cc/oasis7/issues/3650)，仅作为该决策的稳定 locator，不复制任务状态或宣称决策完成。

这些未决问题不阻塞本专题对玩家语义、原文义务映射和验收边界的采纳；在对应实现输入前必须解决或明确排除受影响范围。

## 8. Non-Goals

- 不新增产品 PRD-ID、第二套 demand goal 状态机、订单簿、自动补货、自动停机或后台改道机制。
- 不把 production receipt、buffer 到达、预测数量、旧 schedule 或 Agent recommendation 解释成 matching delivery、terminal settlement、需求满足或奖励。
- 不规定 batch quantum、配方/产率、损耗、价格、capacity、expiry、queue、settlement 公式、runtime schema、API、Viewer 布局或 Agent 实现。
- 不以本专题的叶子要求和 AC 场景宣称实现完成、当前可玩性通过或公开发行就绪；这些结论需要对应专业和分层证据。
