# 游戏可玩性设计总纲 v0.1  

- 对应设计文档: `doc/game/gameplay/gameplay-top-level-design.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-top-level-design.project.md`

审计轮次: 4


## ROUND-002 主从口径
- 本文件为 gameplay 主文档，其余 gameplay 专题为增量子文档。

## —— 让玩家快乐，并愿意长期沉浸的系统设计

> 本文档目标：  
> 在宏大的 AI 文明模拟与区块链架构之上，建立一个真正“好玩”的游戏体验结构。  
> 核心目标不是技术先进，而是：**玩家快乐、持续上线、形成社交与情绪投入。**

---

# 0. 设计摘要（必备字段）

## 0.1 目标

- 建立可长期运营的可玩性框架，让玩家在日常决策中持续获得反馈与成就感。
- 将“战略、工业、经济、协作、治理”整合为统一体验曲线，避免系统堆砌但不好玩。
- 为后续工程落地（WASM 模块化玩法层）提供玩家体验约束与验证基准。

## 0.2 范围

### In Scope
- 玩家体验目标与动机模型。
- 微循环/中循环/长循环的节奏设计。
- 工业、协作与治理的玩法原则与平衡约束。
- 新手前 30 天体验路径与长期沉浸机制。

### Out of Scope
- 具体链上协议实现细节与共识算法实现。
- 底层物理内核/渲染系统/网络协议的代码级设计。
- 数值表、产消参数、伤害公式等实现期调优细节。

## 0.3 接口/数据

- 玩家输入接口：目标设定、提示词注入、治理投票、协作协议、资源分配决策。
- 世界反馈接口：事件日志、组织状态、资源/能源状态、供给变化、提案状态。
- 核心体验数据：
  - 玩家成长数据（组织规模、治理权重、区域控制度）。
  - 工业成长数据（原料/半成品/制成品库存、产线稳定度、工厂开工率、停机原因分类）。
  - 协作数据（协议状态、供给关系、关键协作历史）。
  - 社交治理数据（组织关系、提案通过率、协作调整事件）。

## 0.4 里程碑

- M0（设计冻结）：完成可玩性顶层目标、范围、风险定义。
- M1（原型验证）：闭环验证微循环可玩性（5~15 分钟内有有效决策和反馈）。
- M2（中期验证）：完成工业/协作/治理中循环验证（1~3 天内可达阶段成果）。
- M3（长期验证）：形成文明级目标驱动和历史沉淀（数周级别持续投入）。

## 0.5 风险

- 规则复杂度过高导致新手理解成本过大，首周流失升高。
- 宏系统过早曝光，可能压过工业与控制感主线，削弱早期留存。
- 政治系统若缺乏反制机制，可能出现长期垄断并降低参与感。
- 长期目标若反馈过慢，会导致“努力无感”，降低回访率。
- 如果前期工业成长只表现为抽象数值上涨，而没有“首个制成品/工厂运转”的可见成果，新手会难以确认自己是否真正推动了世界。
- 如果一次间接控制可以长时间停留在“已接受但无主因果、无 fallback、无升级建议”的灰区，玩家会快速退化为被动旁观者。

---

# 第一部分：可玩性方法论

## 1.1 可玩性的底层公式

好玩 =  
（清晰目标）  
+（有限资源）  
+（真实风险）  
+（可见成长）  
+（信息不完全）  
+（人与人博弈）  
+（适度不可预测）

如果任何一个长期缺失，玩家流失。

---

## 1.2 三层游戏循环（Game Loop）

### 一、微循环（5~15 分钟）

- 查看世界变化
- 查看库存、制成品与工厂状态
- 阅读 Agent 行为日志
- 发现问题/机会
- 下达新目标
- 预测结果
- 等待反馈

设计要求：
- 每次上线都有“变化”
- 有可调整的决策空间
- 有即时风险与机会
- 能明确知道产线是否在推进、哪里停机、下一步该修什么
- 任一关键意图在一个 bounded-response 窗口内都必须被归类为 `执行中 / 被阻塞 / 被改道 / 已完成但无有效进展 / 已完成并形成进展` 之一，不允许长时间停留在“像是在做事但我不知道发生了什么”。
- 如果当前意图没有带来有效推进，系统必须给出最小 next-decision surface：`继续等待`、`修复阻塞`、`改道到次优动作`、`没有安全 fallback，重新定目标或升级约束` 四者至少其一；若进入 `fallback_ready`，还必须展示 `fallback_tradeoff_preview`，说明 `blocked_intent_id`、`blocker_reason`、`wait_option`、`repair_option`、`reroute_option`、`progress_kept`、`opportunity_cost`、`recommended_fallback_action_id`、`fallback_value_class`，并把推荐分类为 `safe_wait / repair_now / reroute_now / no_safe_fallback`。
- `safe_wait` 不能只是一条“继续等待”的标签；它必须附带 `wait_resolution_quote`，最小字段为 `resolution_trigger`、`expected_wait_class`、`next_recheck_tick_or_event`、`state_change_expected`、`wait_risk_if_unresolved`、`alternate_action_unlock_condition`。只有当 `resolution_trigger` 来自 canonical world/runtime truth，且当前替代动作不可用或其已披露机会成本更差时，系统才可推荐 `safe_wait`；若不存在有界触发条件，或到了 `next_recheck_tick_or_event` 仍未发生预期状态变化，则必须重新分类为 `repair_now / reroute_now / no_safe_fallback`，不得静默延长等待，也不得伪造精确 ETA。
- `no_safe_fallback` 是透明的终止分类，不是无动作死路；它必须提供 `no_safe_fallback_reason`、`required_next_decision_action_id` 与 `required_next_decision_class`，其中下一决策仅允许 `reprioritize_goal / escalate_constraint / return_to_goal_selection`，让玩家明确结束当前 blocked intent 并进入新的可操作决策面。
- 该 fallback 合同只要求玩家能比较等待、修复、改道的最小取舍，并能回答“在等什么、何时复查、何时换方案”；不扩展为完整恢复系统、quest tree、respec 系统或 UI 重写。
- 如果玩家/API/agent 请求当前未开放的细粒度物理动作（例如挖掘、放块、跳跃、攻击或 local physics），不能只返回 `unsupported_action`；必须展示 `fine_grain_action_translation`，用 `show / compare / classify / recommend` 说明当前动作粒度边界、最接近的间接控制目标和下一步。最小字段为 `requested_granularity`、`why_fine_action_deferred`、`canonical_replacement_action`、`closest_playable_goal`、`player_next_step_hint`、`replacement_value_class`，分类固定为 `replacement_available / no_safe_replacement / future_embodied_candidate`；该合同不开放 block editing、直接具身控制、完整 3D UI 或 runtime 物理实现。

---

### 二、中循环（1~3 天）

- 做出首个制成品
- 建成首条稳定生产链
- 落成首座工厂单元
- 推动一项组织协议通过
- 完成一次协作协议调整
- 完成一次治理或供给取舍

设计要求：
- 有阶段性成果
- 有清晰可见进展
- 有阶段奖励或影响力提升

---

### 三、长循环（数周）

- 控制关键区域
- 获得更多治理权
- 建立大型组织
- 推动基础协议升级

设计要求：
- 有“文明级”目标
- 有权力争夺空间
- 有不可逆历史事件

---

# 第二部分：玩家体验设计文档

## 2.1 玩家角色定位

玩家不是操作者。

玩家是：

> 文明的战略引导者。

玩家不能直接控制 Agent 行动，但可以：

- 指定目标
- 提供提示词
- 引导模块开发
- 影响组织结构

---

## 2.2 玩家核心动机

1. 成长感（组织规模扩大）
2. 权力感（影响协议与规则）
3. 博弈感（与其他玩家较量）
4. 创造感（模块与制度创新）
5. 归属感（组织与协作关系）
6. 影响力（改变世界结构）

---

## 2.3 玩家每日关注点

- 我的能源是否稳定？
- 我的制成品产线有没有断料或停机？
- 是否有人渗透？
- 是否要扩张？
- 是否需要谈判？
- 是否要推动治理提案？
- `PublishSocialFact` / `DeclareSocialEdge` / `ChallengeSocialFact` 若影响谈判、合作、黑名单、治理或 claim 表面，必须展示 `social_fact_impact_quote` / `relationship_consequence_preview`：影响对象、可见社交表面、合作机会变化、争议/stake 风险、治理/claim 关联和推荐社交动作；玩家不应只看到事实 ID、关系边或事件日志。

---

## 2.4 情绪节奏设计

上线 → 紧张  
决策 → 思考  
执行 → 期待  
结果 → 兴奋或焦虑  
修正 → 成就或复仇欲

---

## 2.5 前期工业引导成就闭环

前期引导不应该围绕“造了一个建筑”，而应该围绕“建立了持续运转的组织能力”。

在当前世界观下，玩家作为文明的战略引导者，最早期、最可感知的一组成就应围绕工业成长展开：

1. 首个制成品：首次跑通“原料 -> 加工 -> 产出”，证明玩家已经让世界发生可见变化。
2. 首条稳定生产链：连续多个 tick 保持产出，不因缺电、缺料或物流阻塞中断。
3. 首座工厂单元：把一次性加工升级为持续组织能力，形成长期产能。
4. 首个可交易工业品：第一次感受到分工、交换与协作的价值。
5. 首个受保护工业节点：第一次为产能配置治理、维护或协作保障。

设计要求：
- 每个里程碑都必须有世界状态变化与 Viewer 可见反馈。
- 反馈必须区分 `已接受`、`执行中`、`已产出`、`停机/阻塞` 四类状态。
- 停机必须给出最小原因分类：缺电、缺料、物流阻塞、治理限制、危机或维护窗口影响。
- 奖励应优先体现“能力解锁/组织成长”，而非纯数值奖励。
- 工业里程碑必须自然通向协作、治理、危机与扩张取舍，而不是独立支线。
- 工业成长反馈必须优先展示“新能力 / 新选择 / 新修复手段 / 新区域用途”，而不是只展示库存与产量上涨。
- 首个制成品、首条稳定产线与首个可交易工业品都必须绑定最小经济可读性：玩家应能看懂 `投入了什么 / 产出了什么 / 为什么值得继续 / 下一步可换来什么`。
- 首局推荐采集目标必须把 `target_frag_id / expected_material_hint / starter_value_reason / first_recipe_relevance` 接到第一工业目标；玩家应知道“为什么先采这个 frag”，而不是只看到最近可采集物。
- 高负载工厂或维护 sink 影响首条稳定产线时，玩家的提交前取舍必须分成两条不能互相冒充的信号：`ScheduleRecipe` 当前的 `electricity_cost / electricity_after` 与 `runway_before_ticks / runway_after_ticks / downtime_threshold_ppm / continue_production_risk` 是排程账本可负担性与 Agent idle-battery 安全信号；它不代表工厂维护 runway。当前排程不扣 battery，所以两个 runway 值相等；`maintenance_pressure_delta = unchanged` 也不构成维护消耗、折旧变化或“可安全继续生产”的证据。因而仅当 player-facing surface 已暴露该 quote 时，玩家才能据此在 `restore_power_before_scheduling` 与 `schedule_now` 之间决定；当前尚无 Viewer/LLM 闭环证据，也不能把 `recommended_maintenance_action` 误读成已经可用的维修系统。
- 当 runtime 以后引入可扣减、可恢复且会阻断工厂的维护真值时，`factory_maintenance_status` / `schedule_quote` 必须另外给出 `maintenance_runway_before_ticks / maintenance_runway_after_ticks / maintenance_downtime_threshold / maintenance_pressure_delta / maintenance_failure_cost / recommended_maintenance_action`。玩家应能比较“先维护、降载后排程、带风险继续、暂缓”的时间成本、资源成本、保留产出与停机损失：维护的奖励是保住稳定产线和下一次交付能力，带风险继续的失败成本是明确的产出/时间/恢复损失，而不是只收到事后 receipt。推荐必须指向当前最小可执行动作，并说明该动作完成后为何值得继续（恢复交付、解锁下一单或避免当前工业目标倒退）。没有维护真值时，surface 必须显示 `maintenance_not_tracked` 或不展示维护 runway，不能以 `0`、无限值、battery runway 或“unchanged”伪造安全。
- 平衡与防滥用：quote 必须只读、确定且不推进 maintenance、排程、tick 或资源账本；它解释既有成本而不重平衡折旧、维修价格、产率或停机阈值。首个工厂的冷启动不得因未实现 maintenance 而被强塞虚构维护门槛；一旦 maintenance 已生效，低 runway 推荐必须给出可恢复动作而非无出口拒绝。验收分层为：(1) 当前 runtime 回归证明电力账本与 battery runway 的独立性、`maintenance_pressure_delta = unchanged`，且不宣称 maintenance cover；(2) 未来 runtime 回归证明维护前后字段、临界/停机、降载与恢复建议来自同一权威维护状态并且 quote 不变更状态；(3) Viewer/LLM playtest 让玩家在至少一个正常与一个临界维护场景中说出动作、即时保留收益、失败成本和下一步交付动机。当前只满足第 (1) 层，不得据此宣称 Viewer/LLM 或维护闭环完成。
- `RefineCompound` / `refine_compound` 若作为首个工厂或首个制成品前置恢复动作，必须在提交前展示 `refine_quote` / `refine_preview`：`compound_mass_g`、`electricity_cost`、`hardware_output`、`electricity_after`、`hardware_shortfall_before`、`hardware_shortfall_after`、`first_goal_relevance`、`recommended_refine_amount`、`refine_value_class`；该合同必须让玩家看见投入与产出、比较目标缺口变化、分类为 `enough_to_advance / partial_progress / poor_power_tradeoff`，并推荐继续精炼、先补电或改走采矿/等待路线。它只约束提交前可读性，不重平衡精炼公式、电力成本、产率，也不扩展为完整加工链。
- `market_quotes` 若影响排产或材料采购，必须展示 `market_quote_decision_preview`：`recommended_source`、`local_vs_world_cost_delta`、`tax_contribution`、`transit_contribution`、`remaining_shortfall`、`cost_pressure_class`、`recommendation_rationale` 与 `next_cost_reduction_action`；推荐结果可落到本地采购、外部调运、延后、治理调整或拆分来源，玩家不应只看到 `effective_cost_index_ppm`。该合同只补玩家提交前的来源取舍，不扩展订单簿、撮合交易或市场数值重平衡。
- `TransferMaterial` 若影响首条稳定产线或当前配方阻塞，必须展示 `logistics_transfer_quote` / `transfer_impact_preview`：预计到达量、损耗、到达 tick、优先级理由、吞吐占用、调运前后阻塞变化和推荐调运动作；玩家不应只在 `MaterialTransitCompleted` 后才发现这批材料是否赶上产线。
- 数据采集、跨 Agent 数据转移或数据合约若受电力成本或访问许可约束，提交前必须说明预计电力成本、数据 owner、recipient/use、许可状态与拒绝后的授权或替代路径；玩家不应只看到 access denied，也不得为提升可读性绕过 owner consent。
- `ValidateProductWithModule` / `ProductValidated` 现在通过 `task_c0177461965146a8a1f7bfb99caf9b16`（GitHub #2599）提供已签名、只读且提交前可重复请求的 `product_validation_quote`，并在 Viewer 的 quote card 展示 `product_id`、用途/战略角色、可交易性、验证前后阶段、解锁/价值等级、推荐行动、缺失前置与可达推进/恢复路径；提交后的 `validation_unlock_preview` 继续解释已验证产品的用途与下一步。quote 不提交验证、不执行任意 WASM 模块、不生成 receipt 或改变权威状态。阶段前提在 quote 中是建议；除非 runtime 的实际提交规则阻止，不能把它描述为已禁用提交。该闭环不新增科技树、成就系统、产品链或数值平衡。
- `BuyPower` / `harvest_radiation` / 等待发电若用于恢复低电、临界电力或停机风险，必须展示 `power_survival_quote` / `energy_recovery_preview`：补电量、成本、恢复后状态、可行动 runway、下一步动作可负担性、防停机原因和推荐补电动作；玩家不应只看到缺电拒绝或补电事件。
- `SellPower` 若用于短期变现或缓解现金流，必须展示 `power_sale_quote` / `energy_liquidity_preview`：售电量、预期收入、售电后状态、剩余 runway、下一动作可负担性、产线中断风险和推荐售电动作；玩家不应只看到卖电收入而不知道是否牺牲能源稳定。
- `FragmentsReplenished` / 运行期 frag 补种若影响缺料恢复或第一工业目标，必须展示 `resource_replenishment_quote` / `fragment_refill_preview`：当前 frag/chunk 剩余量、下一次补种 tick、预计补种量、等待成本、第一工业目标关联和推荐资源行动；玩家不应只在后台补种事件后才知道该不该等、换目标或改路线。

## 2.6 PostOnboarding 阶段承接

首次行动闭环完成后，系统不能只留下“一次性总结 + 继续探索”的静态提示，而必须进入正式的 `PostOnboarding` 阶段。

该阶段的目标不是继续教按钮，而是把玩家从“会操作”切换到“建立持续组织能力”。因此：

1. 第一个阶段主目标默认应围绕工业成长、产线恢复或组织能力稳定化。
2. 主目标必须同时展示进度、主要阻塞和建议下一步。
3. 玩家达成首个持续能力里程碑后，系统必须显式展开中循环方向，如生产扩张、治理影响或协作保障。
4. `10-minute trust gate` 只证明玩家已经信任“控制是可靠的、目标是可读的、继续玩是值得的”；`first capability gate` 再证明首个持续能力在后续 `15~45` 分钟或 `1~3` 次会话内闭环，不得把两层 verdict 混写成一个。
5. `PostOnboarding` 默认不允许把玩家丢回“自由漂浮观察态”；如果当前主目标暂不可达，系统必须切到恢复、保全或替代胜利，而不是只保留世界状态观察。

产品承诺与组合验收见 `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`；本节继续拥有阶段选择、阻塞分类与玩法承接的专业规则。

## 2.7 当前两周收口重点（2026-04-09）

在当前 `internal_playable_alpha_late` 阶段，当前 gameplay scope 必须继续冻结在 early-retention recovery，而不是重新发散到更宽的宏系统曝光。

当前短周期 focus order 固定为：

1. 恢复并守住 active-LLM formal lane 的 `10-minute trust gate` 地板，确保首次控制、世界推进、主目标可读与“愿意继续玩”的基础判断重新成立。
2. 收口 `PostOnboarding -> first capability gate` 的正式承接，保证玩家在后续 `15~45` 分钟或 `1~3` 次会话内持续得到 `goal / progress / blocker / next_step`，而不是完成新手后掉回自由漂浮观察态。
3. 把工业链状态、停机原因与修复动作收成 canonical 玩家语义，让玩家能判断“为什么没产出、现在该修哪里”，而不是靠 raw log 或猜测推进。
4. 收口间接控制下的 control-feeling：必须持续给出动作状态、主因果、阻塞分类与最可执行的下一步，避免体验退化成“看 AI 自己决定”。
5. 只有在前 4 项稳定后，才允许重新扩大治理、元进度等宏系统在首局和早期 session 里的曝光或承诺。

范围冻结规则：

- 任何新增玩法提案如果不能直接改善前 4 项之一，默认延后，不进入当前冲刺主路径。
- 如果某项工作主要扩大世界复杂度、卖点数量或系统展示面，但不能降低 early-retention blocker，则必须记为 deferred，而不是与 trust/capability 修复并行抢优先级。
- `--no-llm`、operator-only、Prompt Ops 或其他 debug/probe lane 只能用于排障，不得作为“当前 focus order 已完成”的放行依据。
- 当前 producer 正式口径继续保持双层判定：`10-minute trust gate` 与 `first capability gate` 必须分开记录。2026-04-15 的 `trust gate = hold`、`first capability gate = not_run` 现在只保留为历史 baseline；当前 fresh truth 已更新为 `trust gate = pass`、`first capability gate = pass`，但这仍不等于可以跳过后续更宽的 release / liveops 边界复核。

### 2.7.1 PRD-GAME-012 稳定 early-retention 合同

产品承诺统一见 [`首局与持续游玩`](../../product/world-rules-core-gameplay/first-session-and-continuation.prd.md)。本节拥有不随短期任务日期变化的 gameplay 专业合同；当前 verdict、task trace 与复跑边界统一见 [`doc/game/project.md`](../project.md) 和 `doc/testing/evidence/`。

#### Gate 与 verdict 隔离

- `10-minute trust gate` 判断首次控制可信、主目标可读、玩家后果可见、阻塞可恢复，以及玩家是否出现继续游玩的基础意愿。
- formal headed Web/UI 首次控制地板要求最近样本的首次成功率 `>= 95%`；依赖手动 reopen/reload 才进入可控态、出现 `control ack timed out without progress`，或发生阶段回退伴随冻结的样本均计为失败，并使 trust gate 保持 `hold`，不能由后续恢复后的成功冲淡。
- `first capability gate` 判断首个持续能力是否在后续 `15~45` 分钟或 `1~3` 次会话内闭环；不得因为它没有在首个 10 分钟完成而把 trust gate 判为失败。
- `progression_pass`、`attraction_pass`、`motivation_density_pass` 与 `content_volume_pass` 是四个独立结论。目标覆盖、世界推进、first capability pass、动机密度和内容量不得互相代签。
- formal lane 能推进但缺少新选择、奖励、玩家因果或回访理由时标记 `progression_pass_but_attraction_weak`；动机密度已通过但有效内容量不足时标记 `content_volume_weak`。
- active-LLM / headed live 样本与 deterministic-provider-backed required evidence 必须分开。required tier 证明合同、回归和设计充分性；真实玩家留存、生产 provider 体验或 release/playtest claim 仍需 live/provider evidence。

#### 0~30 分钟 beat 合同

| 时间 | 玩家问题与动词 | 必须形成的反馈/收益 | 失败签名与恢复 |
| --- | --- | --- | --- |
| `0~1m` | 我是谁、现在做什么；进入/读取 | 主目标包含动作、完成条件、阶段预期与下一步 | raw operator/debug 噪音抢焦点或 empty world 无可执行入口 |
| `1~3m` | 我的指令生效了吗；`play/step/select/action` | `Player Intent -> World Consequence -> Recovery Move -> Next Move` 控制证明 | rejected/blocked 缺权限、模式、前提或修复动作 |
| `3~5m` | 世界哪里因我改变；查看/对比 | accepted/executing/produced/blocked/cost 可区分，玩家可指出自己的因果 | 只有世界 tick / Agent 活动，标记 `world_activity_only` |
| `5~7m` | 下一步修、扩、等或改道；选择 | 至少一次带收益、代价、阻塞与预期结果的 meaningful decision | 只能无界等待，必须 fallback 或 reprioritize |
| `7~10m` | 小阻塞是否可处理；repair/fallback/reprioritize | blocker taxonomy 与 wait/repair/reroute/no-safe-fallback 中的可执行恢复 | 无恢复或升级解释，标记 `agency_weakened` |
| `8~12m` | 我获得了什么；确认产出 | 可见产出、小胜利、新用途或下一步价值 | 只有库存/吞吐上涨，标记 attraction weak |
| `12~18m` | 能力是否持续；补料/排程/稳定产线 | 一次产出转为可维护、可恢复并打开新选择的能力 | 只有 progress percent，标记 `progression_pass_but_attraction_weak` |
| `18~23m` | 接下来如何变强；expand/repair/specialize/tradeoff | 分支说明即时收益、后续 beat、风险/锁定与回访 hook | 只给路线标签或重复同一循环，标记 `branch_commitment_missing` / `grind_only` |
| `20~25m` | 能否打断或纠偏 Agent；interrupt/reprioritize/correct | 玩家能读到打断、重排、纠偏或 handoff | 只能等 Agent 自行推进，判定 agency 风险 |
| `25~30m` | 为什么下次回来；选择下一目标/回访包 | 本局成果、choice memory、新 leverage、下一目标和 first action on return | `forced_major_power_dependency` 或 `world_activity_only` 不能算 pass |

#### 内容量、动机与 anti-script 判据

- content-volume 最低门槛固定为：`effective_play_minutes >= 30`、`player_operation_count >= 18`、`content_unit_count >= 8`、`distinct_action_family_count >= 6`、`passive_wait_share <= 0.25`。未达标即 `content_volume_weak`。
- motivation/attraction evidence 至少记录 `hook_score`、`replay_intent`、`meaningful_decision_count`、`reward_or_unlock_count`、`stall_or_wait_periods`、`biggest_boredom_point`、`continue_reason` 与 `return_hook`；这些是 evidence DTO，不得写成 runtime 世界真值。
- `route_tradeoff` 必须影响后续至少 2 个 beat；`accelerate` 与 `stabilize` 至少在中途可见指标、事故后果或回访目标上不同，否则标记 `route_tradeoff_fake_choice`。
- 微型委托必须产生可见、可命名、可交付的成果，并推进同一 `local_demand_id` 的 before/after 进度；只有 reward ID 或静态成果卡不能通过。
- 第二局首屏必须能恢复上一局 choice memory，并据路线、成果或修复选择生成不同的 `next_session_goal` 与 `first_action_on_return`。
- 连续 `step/wait/refresh` 等被动 CTA 即使推进进度，也必须由 boredom negative regression 判为 attraction weak；`quick_patch` 与 `root_cause_fix` 必须展示时间、保留进度、残留风险或稳定收益上的真实差异。
- required summary 必须标注 `runtime_backed / viewer_fixture_only / visual_only / live_verified` 等 provenance；共享 scenario driver 可用于 deterministic 回归，但 mock/fixture 不得冒充真实 0~30 分钟 live gameplay。

#### 选择可读性与 quote/preview 合同

- `branch_offer`：每条推荐路线必须包含 `route_label / immediate_gain / future_beat_changed / risk_or_lockin / next_session_hook`。
- 路线可回退时必须提供 `rollback_deadline_beat / rollback_cost_summary / rollback_kept_benefit / rollback_lost_benefit`；缺失标记 `route_rollback_quote_missing`。
- `Opportunity Scan` 的未推荐 hook 必须说明 `hook_value_summary / hook_readiness_state / defer_reason / unlock_precondition / revisit_timing_hint / value_vs_immediate_action`；缺失标记 `opportunity_discard_reason_missing`。
- starter frag 推荐必须说明粗粒度材质预期、可达性与第一工业目标关联，不承诺精确掉落或必然完成能力链。
- `ScheduleRecipe` 必须在提交前展示基础耗时、本地稀缺延迟和当前电力/battery 风险；仅当 runtime 存在权威维护状态时，才另外展示维护 sink、折旧压力、维护前后 runway、维护停机临界点与维护建议。电力/battery runway、`maintenance_pressure_delta = unchanged` 或执行后 receipt 均不能替代维护 quote。
- `RefineCompound` 必须展示质量、电力成本、hardware 产出、精炼后电力、目标缺口前后变化、推荐量与 `enough_to_advance / partial_progress / poor_power_tradeoff`。
- `market_quotes`、`TransferMaterial`、`ProductValidated` 分别必须解释采购来源与税费/运输成本、调运到达量/损耗/ETA/产线阻塞变化、验证后的用途/可交易性/能力解锁/下一步。
- `BuyPower`、`harvest_radiation` 或等待发电必须解释补电量、成本、恢复后状态、可行动 runway 与防停机收益；`SellPower` 必须解释收入、售电后 runway、下一动作可负担性和停机风险。
- `FragmentsReplenished` 或运行期补种必须解释当前余量、补种触发/时机/数量、等待成本、第一工业目标关联，以及等待、转移或替代材料路线。

#### TASK-GAME-076 自动化与证据边界

- required 入口保持 `./scripts/verify-gameplay-attraction-automation.sh --tier required`；live 玩家路径与 pure API 证据使用 `./scripts/verify-gameplay-attraction-automation.sh --tier live`。
- required tier 必须覆盖 scenario summary writer、weak-sample regression、route branch、second-run hook、anti-script、boredom negative 和 truth/provenance coverage；live tier 才能把真实 browser/player-path 或 pure API gameplay 标为 `live_verified`。
- 当前 required evidence 为 `attraction_pass`、`motivation_density_pass`、`content_volume_pass`，记录 `34/30` 分钟有效内容与 `22/18` 次玩家操作；它不等于真实玩家留存或生产 provider 放行。
- runtime/viewer/agent canonical truth 或对应 surface 发生变化时，重跑 required；需要真实体验、provider 或 release/playtest 判断时，再跑 live/provider sample 并由 QA/producer 分别给出验证与阶段判断。

## 2.8 物理世界尺度与玩家交互尺度

oasis7 的世界不是无尺度表格。

## 2.9 成熟世界中的小玩家成长线

在 `PostOnboarding` 与首个持续能力之后，产品还必须回答另一件事：当世界已经存在更强组织、更深政治和更长历史时，小玩家/新玩家为什么还值得继续玩。

当前答案不应是“立刻加入大组织”，也不应只是“世界本来就很热闹”。正式路线应当至少提供 1 条不依赖立即站队的 `small-player lane`，让玩家能在成熟世界里继续形成独立 leverage。

当前冻结的默认主线是：

1. `local operator`：先稳住 1 条小规模工业或服务能力，完成 1 次对世界有可见后果的胜利。
2. `regional specialist`：再把这条能力转成短周期、区域性有用的专业化角色，而不是马上跳到全局治理或大型宏系统。
3. `limited-scope regional influence`：通过持续贡献获得局部优先级、局部机会或局部可见度，但不直接等价为 global governance 权力。

从 `local operator` 切到 `regional specialist` 之前，系统必须展示 `specialization_entry_quote` / `first_delivery_preview`：玩家要知道候选专业化的第一单交付会满足哪个本地需求、预计产出什么、需要哪些输入、多久形成价值、解锁哪种 `leverage_class`，以及交付后的回访 hook。否则专业化只是抽象标签，不能证明 mature-world 小玩家仍有可判断的经营取舍。

每个 small-player lane checkpoint 还必须展示 `leverage_checkpoint_summary`：`checkpoint_id`、`previous_leverage_class`、`new_leverage_class`、`new_option_unlocked`、`regional_usefulness_delta`、`recovery_resilience_delta`、`negotiation_position_delta`、`same_loop_repeat_count`、`grind_risk_reason`、`recommended_next_branch`、`leverage_checkpoint_class`。该 summary 需要把结果分类为 `new_option_unlocked / resilience_improved / negotiation_position_improved / regional_usefulness_increased / grind_only`；如果只展示 throughput、库存或同一产线重复执行，而没有新选择、恢复弹性、议价位或区域用途，不能判定为 small-player lane progression。

这里所谓 `protected first industrial win`，保护的不是“不会被碰”，而是：

- 早期 footprint 小，不应一开始就与 major-power 主战略面重叠。
- 失败后存在 repair / rebuild / pivot 路径，不会立刻把玩家打回“只能投靠别人或只能退坑”。
- 玩家必须能明确回答“我做了什么、世界因此变了什么、下一步为什么仍值得继续”，而不是只看到世界自己在运转。
- 这条线不能只靠“再多做一点同样的工业”维持；每一阶段都必须新增一个 leverage class，例如更稳的恢复权、更短的交付周期、更有议价能力的局部服务位，或新的区域性选择权。
- 如果继续玩唯一能得到的只是更高产量、但没有新的局部用途、恢复弹性或选择空间，这条线应判定为 grind-only，而不是 mature-world lane 成立。
- 如果专业化推荐缺少第一单交付预览，应标记 `specialization_delivery_preview_missing`，不得只用 `recovery_operator` / `conversion_specialist` / `regional_service_runner` 标签替代玩家侧收益说明。

这条线与当前 `PRD-GAME-012` 的 early-retention 冲刺边界保持分离：

- 当前 trust gate / first capability gate 仍是最近两周主优先级。
- `#165` 解决的是“首个持续能力之后如何继续有独立价值”，不是重新改写首个 10 分钟。
- 只有当成熟世界下的小玩家样本能持续给出 `player leverage != world activity only` 的证据时，这条线才算正式成立。

产品承诺见 [`doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`](../../product/world-rules-core-gameplay/mature-world-progression.prd.md)；本节拥有 lane、checkpoint、专业化与 anti-grind 的玩法合同。

Agent-facing 专业执行合同继续消费 `small_player_lane_id`、`leverage_class`、`same_loop_repeat_count`、`grind_only_flag`、`major_power_dependency_status`、`recovery_path_kind`、`requires_major_power_sponsorship` 以及 repair / rebuild / pivot 可用性，并保留 `selected_specialization_id`、`specialization_reason`、`preferred_next_action_class`、`dependency_boundary`、`recovery_escalation_reason` 等可解释摘要；若 guardrail 改写决策，还必须保留 `decision_rewrite` receipt。执行顺序默认先维持 `local_operator`，再按区域需求进入恢复、转换或区域服务专业化；只有玩家自愿升级，或 runtime 明确标记独立路线不可行时，才允许把 major-power dependency 作为有原因的升级路径。当 `same_loop_repeat_count >= 3` 且 leverage 仍是 `throughput_only` / `unclassified` 时，必须停止强化同一循环。

以下情况属于 blocker：独立路径可用时仍把 sponsor / alliance 写成必需；`grind_only` 后继续强化同一 throughput 循环；第一项专业化直接跳到全局治理、联盟领导或战争。没有 bounded canonical trigger 和复查时机时，`wait / wait_ticks` 不能替代 repair / rebuild / pivot。专业化预览缺少本地需求、第一项产出、交付时机或 leverage 解锁时，继续标记 `specialization_delivery_preview_missing`。

当前正式口径必须同时成立两件事：

1. 世界物理真值有尺度，而且当前 canonical 空间坐标、距离与尺寸继续以厘米整数合同落地。
2. 玩家当前默认玩的不是 Minecraft 式第一人称逐块编辑，而是“通过目标、动作链和组织能力间接改变一个有物理尺度的世界”。

因此需要区分四层尺度语义：

1. `canonical physical scale`
   - 用来回答“世界真实存的是什么”。
   - 包括位置、距离、半径、尺寸等厘米真值。
2. `subsystem native resolution`
   - 用来回答“某个子系统实际按什么粒度工作”。
   - 例如 chunk / voxel / location / facility 可以比厘米更粗，但必须声明映射规则。
3. `player interaction scale`
   - 用来回答“玩家当前默认改世界，是通过什么粒度的动作”。
   - 当前正式主路线仍是 `agent/location/facility/recipe/governance` 这类间接控制动作，而不是 block placement / digging / local terraforming。
4. `presentation scale`
   - 用来回答“Viewer 为了可读性放大了什么”。
   - 视觉夸张允许存在，但不能冒充物理真值。

这意味着：

- `1cm` 是世界真值合同，不自动等于“玩家已经拥有 1cm 级直接操作主玩法”。
- coarse-grained 子系统不是违背尺度设计，而是需要被正式声明和审计。
- Viewer 的 marker 放大、semantic map、2D 抽象不等于世界尺寸改变。
- 过细动作只能映射到 canonical 可执行替代动作；没有安全替代时，必须安全停止、解释边界并给出下一次可决策点，不能由 UI 伪造动作或只返回无解释失败。
- 未来如果要引入 embodied / block-editing 候选能力，必须先证明它强化的是当前间接控制文明模拟主路线，具备对应专业域合同与验证，并经显式跨域决策；不得把产品切成第二套游戏或提前写成当前承诺。

产品承诺见 [`doc/product/world-rules-core-gameplay/prd.md`](../../product/world-rules-core-gameplay/prd.md)；本节保留玩法侧四层尺度合同与未来候选 guard。

补充口径：
- `10-minute trust gate` 负责回答“是否已经值得继续玩”。
- `first capability gate` 负责回答“首个持续能力是否已经闭环”，并继续与 `PostOnboarding` 的 `15~45` 分钟里程碑口径对齐。

## 2.10 间接控制下的 control-feeling 合同

oasis7 当前正式主路线不是 direct control，而是 indirect control。

这不等于玩家应该接受“AI 自己做决定，我只看结果”。

当前正式口径要求，玩家在间接控制主路线里仍然必须持续回答 4 个问题：

1. 我刚刚让系统做了什么。
2. 系统有没有接受这件事。
3. 为什么现在这样推进、没推进或改道。
4. 我现在最有效的下一步是什么。

如果任一问题不能在 headed Web/UI 或 pure API 的正式玩家 surface 上被直接回答，那么即便世界还在推进，也不能视为 control-feeling 合格。

因此当前 gameplay 正式冻结 4 条 guarantees：

1. `accepted intent`
   - 玩家必须能读到当前被接受的主意图，而不是只看到原始事件流。
2. `execution causality`
   - 玩家必须知道当前是执行中、被阻塞、被 override、无进展完成还是有进展完成。
3. `interrupt / reprioritize / recover`
   - 玩家必须能打断、重排或恢复 agency，而不是只能等 AI 自己继续。
4. `bounded consequence readability`
   - 玩家必须看懂当前代价、世界变化、主阻塞与最短下一步。

这 4 条 guarantees 的专题合同见 `doc/game/gameplay/gameplay-indirect-control-agency-contract.prd.md`。

当前 defer 范围：
- 暂不继续扩大高风险对抗/治理/元进度在首局中的曝光。
- 暂不把新的宏系统入口包装成 first-session 主卖点。
- 暂不把 `--no-llm` 调试 lane 重新定义为正式游玩入口。
- 暂不把 Prompt Ops / operator-only 入口作为默认玩家主路径。

early-retention 产品承诺见 `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`，专业玩法合同见本文件 2.7.1，当前 verdict 见 `doc/game/project.md` 与 `doc/testing/evidence/`。

## 2.11 纯 API 客户端等价

纯 API 客户端不应该只是“能看日志、能发 step”的调试通道，而应是正式玩家入口之一。

设计原则：

1. 纯 API 与 Web/UI 必须共用同一套世界事实、阶段目标、阻塞解释和下一步建议。
2. 允许表现形式不同，但不允许信息粒度和动作能力降级。
3. 如果 UI 能告诉玩家“我现在在哪个阶段、为什么被卡住、下一步该做什么”，那么 API 也必须能告诉玩家。
4. 如果 UI 能继续玩到中循环，API 也必须能继续玩到中循环，而不是停留在首局或探针模式。

产品模式、证据隔离与发行组合验收见 `doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md`；本节继续拥有 pure API 持续玩法等价的专业规则。

## 2.12 Future：玩法 mode 的参与与准入可读性（未实现）

这是后续目标，不改变当前 `GameplayModeReadiness`、runtime 或 UI 行为，也不承诺匹配、排队、邀请或自动补人。未来若某个 surface 暴露 mode 进入，必须把**当前参与状态**与**这次准入**分开：对同一 mode 的所有 active gameplay module，`(min_players, max_players)` 必须完全相同（`min_players == max_players` 是合法固定人数 mode）；任一边界不一致即 fail closed，不做 min/max、交集或其他聚合。

- 当前参与状态只说明已有玩家：`current_players < min_players` 为 `waiting_for_players`，否则最低人数已满足；有限上限且 `current_players >= max_players` 时同时为 `full`。它不决定最后一个空位能否被准入。
- 准入是 prospective：模块 ready 且有限上限满足 `current_players < max_players` 时可准入，故即使当前仍 `waiting_for_players`，最后一个空位也可被加入；无上限时只要模块 ready 即可准入。`current_players >= max_players` 必须拒绝为 `full`。
- `enterable` 只能由“模块 ready + prospective 可准入”得出。模块未 ready 时必须明确模块/缺失原因，不得伪报为等待或满员；等待时必须说明仍缺人数但不虚构 ETA；满员时必须说明容量原因并给出返回其他当前可进入 mode 的恢复路径。人数重判不得隐式替玩家加入或退出。

---

# 第三部分：爽点曲线模型

## 3.1 爽点来源分类

### 1. 控制感

- 目标被成功执行
- 协议调整或供给安排取得预期效果

### 2. 逆转感

- 劣势翻盘
- 组织重组后崛起

### 3. 创造感

- 新模块被广泛使用
- 自己的制度成为标准

### 4. 区域影响感

- 区域控制
- 资源垄断

### 5. 危机生存感

- 风暴中维持能源
- 危机中保住核心设施

---

## 3.2 爽点曲线结构

### 阶段 1：探索兴奋期
- 快速成长
- 学习系统
- 小规模成功

### 阶段 2：压力增长期
- 供给压力增多
- 风险变高
- 决策复杂化

### 阶段 3：政治博弈期
- 联盟形成
- 谈判博弈
- 权力争夺

### 阶段 4：文明级决策
- 推动协议升级
- 参与治理
- 影响世界规则

---

# 第四部分：协作与治理系统设计

## 4.1 协作设计原则

- 协作必须有成本
- 协作必须有可审计承诺
- 协作必须能解释收益与风险
- 协作不能退化成单一最优路线

---

## 4.2 协作形式

### 供给协议
为能源、材料、制成品和维护能力建立短期或长期供给关系。

### 资源共享
在组织、区域或项目目标之间分配资源和维护窗口。

### 治理提案
通过提案调整规则、优先级和风险承担方式。

`OpenGovernanceProposal` / `CastGovernanceVote` / 改票若影响治理提案结果，必须展示 `governance_vote_quote` / `proposal_outcome_preview`：剩余时间、quorum/pass 缺口、玩家票权影响、可能结果变化、通过后的规则/优先级变化、失败或冷却代价和推荐治理动作；玩家不应只看到提案日志、阈值参数或最终结算。

### 战争宣告
`DeclareWar` / 调整战争强度若影响联盟冲突，必须展示 `war_declaration_quote` / `conflict_outcome_preview`：双方联盟、推荐强度、持续时间、双方评分估计、预计胜负、冲突窗口占用、重入阻塞、结算风险、替代行动和推荐宣战动作；玩家不应只看到宣战事件或最终战报。

### 危机响应
在供应中断、设施故障或链上异常时组织修复、让渡和恢复。

---

## 4.3 协作平衡机制

- 承诺需要资源或信誉成本
- 违约、失约和延期必须留下事件证据
- 治理门槛和冷却窗口限制频繁改约
- 投票影响、策略更新和声誉收益必须经过授权并受反滥用、反刷取和反通胀边界约束；具体权重、阈值与奖励公式由当前 runtime / balance 真值维护，不在产品或玩法总览中冻结历史数值。
- 危机响应优先保护可恢复性，而不是放大不可逆惩罚

---

## 4.4 政治系统

### 组织协作系统

- 多玩家形成组织或协作关系
- 内部协议可制定
- 可投票决策

### 提案机制

- 提交规则变更提案
- 需满足投票权门槛
- 通过后进入升级流程

### 内部分歧

- Agent 内部可能存在分歧
- 组织内部投票可能失败
- 退出、拆分或重新授权可能发生

---

# 第五部分：前 30 天新手体验流程

## 第 1~3 天：生存学习

- 学习能源循环
- 构建第一个稳定发电模块
- 做出首个制成品
- 理解时间消耗机制

目标：建立稳定能源基础，并第一次看到世界被自己加工。

---

## 第 4~7 天：扩张阶段

- 建立第二个能源节点
- 建成首条稳定生产链
- 落成首座工厂单元
- 开始空间移动
- 以低承诺的交易/服务、互助或信息交换方式首次接触其他玩家；是否进入组织或治理必须留到后续显式升级动作

目标：第一次拥有持续工业能力，并在不强制站队的前提下理解合作可能带来的价值；组织、治理或更高承诺的扩张取舍仍需等到玩家主动升级。

### 首次社交接触预览合同

第 4~7 天的首次社交入口必须在玩家确认前提供 `first_contact_preview` / `social_contact_quote`，把“接触其他玩家”收成一个可理解、可延后的下一步，而不是默认加入组织、承担治理义务或开启宏观外交。每个候选接触必须展示：

- `contact_purpose`：本次要解决的当前本地问题，例如出售少量产出、请求一次运输协助或交换路线/价格信息。
- `expected_mutual_value`：玩家与对方各自立即可见的收益，不得只写“建立关系”。
- `risk_or_commitment`：本次最多暴露什么资源、时间、信誉或后续义务；低承诺接触不得隐含组织身份、治理票权、长期供给或排他协议。
- `solo_lane_preserved`：明确完成、拒绝或延后该接触后，`local operator -> regional specialist -> limited-scope regional influence` 的独立路线仍可继续，且不会失去当前工业/服务主目标。
- `recommended_contact_action`：推荐的具体接触动作及其理由，或推荐保持独立推进。
- `defer_reason`：若当前不适合接触，说明可以延后的原因、保留的收益和下一次值得回看的触发条件。

`first_contact_class` 只能取以下枚举：

- `trade_or_service`：一次性或范围受限的交易、服务交换。
- `mutual_aid`：围绕当前 blocker 的可回收互助，不建立持续成员关系。
- `information_exchange`：交换路线、价格、风险或机会信息，不承诺资源、票权或组织身份。
- `defer_contact`：当前独立路线更优或风险不可接受时，保留延后理由与回访条件。
- `organization_escalation`：明确加入组织、承担长期供给、治理或排他承诺；此类动作不是第 4~7 天首次接触的默认结果，必须作为第 8~14 天或之后的独立确认步骤展示。

如果首次社交入口没有 `first_contact_preview`，或把 `trade_or_service`、`mutual_aid`、`information_exchange` 默认升级为 `organization_escalation`，标记 `first_contact_preview_missing`。该合同只定义当前 limited playable technical preview 中的玩家理解与选择边界，不新增组织、治理、外交、社交图谱或 runtime/viewer 实现范围。

---

## 第 8~14 天：组织形成

- 生产首个可交易工业品
- 加入或创建组织协作关系
- 建立资源共享协议
- 完成第一次治理或供给调整

目标：体验工业分工如何外溢为协作与治理。

---

## 第 15~21 天：战略博弈

- 建立中型能源网络
- 参与提案投票
- 尝试区域控制

目标：获得影响力。

---

## 第 22~30 天：文明参与

- 推动协议升级提案
- 组织多玩家行动
- 体验文明级决策

目标：从参与者变成决策者。

---

# 第六部分：长期沉浸设计

## 6.1 不完全信息

- 不公开全部资源数据
- 情报需侦察获得
- 谈判存在欺骗空间

---

## 6.2 周期性危机

- 小行星风暴
- 能源衰减周期
- 链上分叉危机

危机强制合作、让渡或治理取舍。

---

## 6.3 历史记录系统

- 记录重大治理调整
- 记录组织协作关系形成
- 记录工业中心与工厂网络形成
- 记录协议升级

形成文明记忆。

---

# 第七部分：沉迷设计原则

沉迷不是靠数值膨胀。

沉迷来自：

- 持续未完成的目标
- 未解决的供给问题
- 未兑现的协作承诺
- 未通过的提案
- 未稳定的能源网络

玩家必须始终：

> 有事情可以做  
> 有风险可以防  
> 有权力可以争  
> 有取舍可以做

---

# 第八部分：可玩性评审组织方案

## 8.1 评审目标

- 以可操作证据确认微循环（5~15 分钟）、中循环（1~3 天）、长循环（数周）是否可验证。
- 识别“有体验目标但无工程观测”的断层，避免只靠叙事判断可玩性。
- 形成进入下一轮数值和模块迭代前的评审结论、阻塞项与责任人。

---

## 8.2 评审输入包

- 玩法与目标基线：`doc/game/gameplay/gameplay-top-level-design.prd.md`、`doc/game/gameplay/gameplay-engineering-architecture.md`。
- Gameplay 生产落地证据：
  - 玩家侧治理、战争、危机与元进度合同由本文对应章节、`doc/game/gameplay/gameplay-war-politics-mvp-baseline.md` 和当前测试矩阵承接；战争仍不是当前玩家-facing 主线。
  - 生命周期协议、tick 推进、模块 bootstrap/readiness 与 replay/恢复边界由 `doc/world-runtime/prd.md#gameplay-生命周期协议边界` 承接；历史 layer closure 实现过程从 Git history 与 GitHub task evidence 追溯。
  - 模块驱动生产切片的完成状态由本专题 T3、下方测试矩阵、当前代码与回归测试共同承接；原增量 closure 三件套已退役，历史实现过程从 Git history 与 GitHub task evidence 追溯。
- 测试入口与执行规范：`testing-manual.md`（S1/S2/S3/S6/S7）。

---

## 8.3 评审流程（90 分钟）

- 00~15 分钟：回放范围与目标，确认“微/中/长循环”的判定口径。
- 15~45 分钟：演示微循环与中循环样例链路（动作 -> 事件 -> 状态 -> 可见反馈）。
- 45~70 分钟：对照测试与日志验证“可复现性”（非一次性演示）。
- 70~90 分钟：汇总结论、记录阻塞项、指定下一轮 owner 与截止时间。

---

## 8.4 可验证性判定门槛

- 微循环可验证：玩家在单次会话内可完成“观察 -> 决策 -> 反馈 -> 调整”闭环，且对应链路可由 S3/S6 证据复现；对前期工业链路，至少能复现一次“首个制成品”或“停机 -> 恢复 -> 继续产出”样例。
- 中循环可验证：工业扩张、协作、治理或危机恢复至少一条 1~3 天目标链路可重复达成，并能用运行时状态与事件历史解释结果。
- 长循环可验证：30 天路径的阶段目标（组织形成、区域影响、协议推进）存在连续积累证据，不依赖人工临时修正。

---

## 8.5 评审输出模板

- 结论：通过 / 附条件通过 / 不通过。
- 证据：对应测试命令、关键事件、状态快照路径。
- 问题清单：按 P0/P1/P2 标注影响范围与复现路径。
- 行动项：每项包含 owner、计划完成日期、回归测试入口。

---

# 第九部分：爽点曲线量化指标映射

## 9.1 指标设计原则

- 指标必须可由运行时事件直接计算，不依赖人工主观打分。
- 指标必须按“微循环/中循环/长循环”分层，避免只看单一留存。
- 指标必须能对应玩法动作（工业、协作、治理、危机、元进度），用于定位失真环节。

---

## 9.2 核心指标矩阵（v1）

| 维度 | 指标 ID | 计算口径 | 事件数据源 | 目标区间（首轮） |
|---|---|---|---|---|
| 留存 | `retention_d1` | D+1 回访玩家数 / 当日新增玩家数 | 会话日志 + 玩家活跃快照 | `>= 35%` |
| 留存 | `retention_d7` | D+7 回访玩家数 / 当日新增玩家数 | 会话日志 + 玩家活跃快照 | `>= 15%` |
| 前期成长 | `first_finished_good_time_p50` | 新玩家从首次有效输入到获得首个制成品的 p50 ticks | 生产完成事件 + 玩家进度快照 | `<= 1800 ticks` |
| 工业稳定 | `stable_line_rate_24h` | 24h 内至少拥有一条稳定生产链的活跃玩家占比 | 产线状态变更事件 + 库存快照 | `>= 55%` |
| 工厂开工 | `factory_uptime_100ticks` | 工厂单元在最近 100 ticks 内的平均开工占比 | 工厂状态事件 + 工厂快照 | `>= 70%` |
| 停机可解释性 | `production_blocked_reason_coverage` | 停机状态中带明确原因分类的占比 | Viewer/runtime 状态快照 | `>= 95%` |
| 危机触发频次 | `crisis_freq_100ticks` | `CrisisSpawned` / 100 ticks | `DomainEvent::CrisisSpawned` | `1 ~ 6` |
| 危机恢复质量 | `crisis_recovery_rate` | 成功恢复或替代完成数 / 危机触发数 | `DomainEvent::CrisisResolved`、`DomainEvent::CrisisTimedOut` | `>= 65%` |
| 协作活跃度 | `cooperation_active_rate_24h` | 24h 内发生治理投票、经济合约或资源共享动作的组织数 / 组织总数 | `DomainEvent::GovernanceVoteCast`、`DomainEvent::EconomicContractOpened` | `>= 45%` |
| 治理参与度 | `governance_participation_24h` | 24h 内治理唯一投票者数 / 24h 活跃玩家数 | `DomainEvent::GovernanceVoteCast` | `>= 40%` |
| 危机参与质量 | `crisis_resolve_success_rate` | `CrisisResolved(success=true)` / `(CrisisResolved + CrisisTimedOut)` | `DomainEvent::CrisisResolved`、`DomainEvent::CrisisTimedOut` | `45% ~ 75%` |
| 长期成长 | `meta_progress_velocity_24h` | 24h 内人均元进度积分增长 | `DomainEvent::MetaProgressGranted` | `>= 8` |

说明：
- `first_finished_good_time_p50` 用于衡量新玩家多久才能第一次确认“我真的加工了世界”。
- `stable_line_rate_24h` 与 `factory_uptime_100ticks` 共同衡量前期工业成长是否从一次性成功过渡到持续能力。
- `production_blocked_reason_coverage` 直接对应“动作已接受但没有前进”的反馈缺口，必须优先盯。
- `crisis_freq_100ticks` 低于下限通常意味着世界过静态；高于上限通常意味着玩家被持续打断，形成疲劳。
- `cooperation_active_rate_24h` 是“社交治理是否真实发生”的主观测替代指标。
- `crisis_resolve_success_rate` 目标区间设为中间态，避免“永远成功”或“永远失败”。

---

## 9.3 指标回看节奏与告警

- 日看板：`retention_d1`、`first_finished_good_time_p50`、`stable_line_rate_24h`、`production_blocked_reason_coverage`。
- 周评审：`retention_d7`、`factory_uptime_100ticks`、`crisis_recovery_rate`、`governance_participation_24h`、`meta_progress_velocity_24h`。
- 连续 3 天低于目标下限触发玩法回归分析，并要求给出修正动作与回归测试清单。

---

## 9.4 历史战争/政治数值基线边界

- 历史战争/政治最小可行数值基线仍保留在 `doc/game/gameplay/gameplay-war-politics-mvp-baseline.md`，用于 runtime 兼容、历史证据和未来受控重启参考。
- 当前玩家-facing 总纲不再把战争作为主玩法承诺；当前主线优先工业、经济、协作、治理与危机恢复。

---

# 终章

这是一个：

- 战略博弈场
- 政治演化系统
- 文明竞争模拟
- AI 行为生态

真正的成功标准不是 TPS 或链上性能。

而是：

> 玩家是否在凌晨 2 点还在思考：  
> 明天我要不要改掉那条供给协议？

当玩家开始为世界担忧，  
为协作争论，
为协议投票，  
为能源线焦虑，

游戏就活了。
