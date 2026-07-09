# Gameplay 10 分钟留存修复计划（2026-04-09）设计文档

- 对应需求文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md`

审计轮次: 1

## 设计目标
- 把“偶尔好玩”收成“10 分钟内稳定想继续玩”的产品地板。
- 把正式 verdict 拆成两层：先用 `10-minute trust gate` 证明“值得继续”，再用 `first capability gate` 证明“首个持续能力已成立”。
- 用最少的新范围改动，优先解决可信度、目标承接和奖励节奏。
- 让跨角色任务按明确先后顺序推进，避免 viewer/runtime/QA 各自优化却无法叠成玩家体验。

## 设计原则
- 先可信，再扩面：最小控制地板不稳定时，不新增主路径功能。
- 先中循环，再宏系统：工业持续能力先于高风险对抗/治理大扩面。
- 先主目标，再调试语义：首屏必须服务玩家决策，而不是服务排障。
- 先正式 lane，再 debug lane：active-LLM 样本决定正式门禁，`--no-llm` 只服务定向定位。

## 角色切片
- `producer_system_designer`
  - 冻结 5 条 lane 的优先级、非目标与放行条件。
  - 审核 QA 汇总后给出 continue / hold。
- `viewer_engineer`
  - 负责首屏降噪、玩家身份表达、Mission HUD/summary/toast/chatter 的语义收口。
  - 负责正式入口主路径可见反馈。
- `runtime_engineer`
  - 负责 `play/step/select` 的地板语义。
  - 负责工业状态、阻塞、恢复、分支建议的 canonical 可解释性。
- `qa_engineer`
  - 负责 active-LLM 10 分钟 trust 样本与 `software_safe` floor 的统一 verdict。
  - 负责把 capability follow-up verdict 与 trust verdict 分开归档。
- `agent_engineer`
  - 只在反馈语义需要把“Agent 回执”改成“玩家指挥结果”时介入，不扩展 AI 新玩法范围。

## 实施顺序
1. `TASK-GAME-061`: 冻结专题、挂载根入口、停止范围蔓延。
2. `TASK-GAME-062`: 修首连、控制 floor、software_safe 最弱入口。
3. `TASK-GAME-064`: 在地板趋稳后收首屏噪音与主目标表达。
4. `TASK-GAME-063`: 把 `PostOnboarding` 后的工业中循环包加厚。
5. `TASK-GAME-065`: 用 active-LLM 10 分钟 trust gate 做继续/暂停裁决，并把 capability gate 作为后续独立复验。

## 验证口径
- required:
  - 文档互链、任务映射、执行日志。
  - headed Web/UI 主路径复跑。
  - industrial onboarding 卡组与 10 分钟样本。
- full:
  - 正式 active-LLM 样本趋势与弱入口复核。

## 2026-06-25 Follow-up: Viewer Surface Alignment
- `software_safe` 的 `Formal Gameplay Summary` 已追加 `Control Proof`，把 `Player Intent -> World Consequence -> Recovery Move -> Next Move` 放在同一张首局可读链路里。
- P1/P2 的 `Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation` 与 `Share Replay` 只作为 trust gate 之后的 continuation surface，帮助制作人判断能动性、anti-grind 与中循环承接。
- 这些 surface 只消费或派生现有 `player_gameplay` / feedback 真值，不改变 `10-minute trust gate`、`first capability gate`、runtime canonical truth 或 QA verdict。

## TASK-GAME-076: 0-30 分钟吸引力玩法脚本

### 目标边界
- 本节把 `first-10-30-minute-attraction-hardening` 拆成实现可用的小环节；它不新增宏系统、不升级阶段、不替代 `trust gate` / `first capability gate`。
- `can-progress` 只说明玩家能连接、控制、推进到 first capability，并能读到 `goal / progress / blocker / next_step`。
- `wants-to-continue` 必须额外证明：前 `10` 分钟内玩家至少经历“我造成了变化 + 我做了选择 + 我看到奖励或恢复”；前 `30` 分钟内至少出现“新选择 / 新用途 / 新 leverage / 回访理由”。
- `30-minute target coverage` 不等于 `30-minute content volume`：即使 10 个 beat 都能被 Playwright/agent-browser 逐项点通，也必须另有 `content_volume_card` 证明有效内容量。当前最低门槛为 `effective_play_minutes >= 30`、`player_operation_count >= 18`、`content_unit_count >= 8`、`distinct_action_family_count >= 6`、`passive_wait_share <= 0.25`；未达标时只能标记 `content_volume_weak`，不得声称内容足够 30 分钟。
- 如果 formal lane 能推进但缺少新选择、奖励、回访理由或玩家因果感，样本必须标记 `progression_pass_but_attraction_weak`，不得包装成 `attraction_pass`。

### Beat 脚本

| 时间 | 小环节 | 玩家问题 | 玩家动词 | 系统反馈 / surface | 奖励与动机 | 失败 / 恢复 | 主 owner | 验收证据 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `0-1m` | 身份与当前目标落地 | 我是谁？现在该做什么？ | 进入、读取 | 首屏顶部只放 `I am / Goal / Blocker / Next`；Mission / Formal Gameplay Summary 显示身份、主目标、阻塞、下一步 | 降低迷茫，建立“我有位置” | empty world 显示 claim-first-agent 或默认 PostOnboarding 工业目标，不显示 raw operator 噪音 | `viewer_engineer` + `runtime_engineer` | `goal_clarity`；首屏无 debug 抢焦点 |
| `1-3m` | 首个控制证明 | 我点/发出的命令真的生效了吗？ | `play` / `step` / `select` / gameplay action | `Control Proof`: `Player Intent -> World Consequence -> Recovery Move -> Next Move`；提交后从 `not_submitted` 变成 `accepted` | 从“看世界”变成“我能改变世界” | rejected / blocked 必须说明权限、模式或前提，并给可修动作 | `runtime_engineer` + `viewer_engineer` + `agent_engineer` | `first_control_success`、`action_effect_feedback` |
| `3-5m` | 第一次后果读懂 | 刚才世界哪里变了？ | 查看结果、对比 | `World Consequence` + toast/chatter；展示 `accepted / executing / produced / blocked / cost` | 形成因果记忆 | world tick 但无主因果，标记 `world_activity_only` 风险 | `viewer_engineer` + `qa_engineer` | 样本能回答 `what did I cause` |
| `5-7m` | 第一选择钩子 | 下一步我是修、扩、等，还是改道？ | 选择推荐动作 | 推荐动作必须带收益、代价、阻塞、预期结果和可执行 action/template | 有“我在做决定”的感觉 | 只能继续等待超过一轮，必须给 fallback / reprioritize | `gameplay_designer` + `runtime_engineer` + `agent_engineer` | `meaningful_decision_count >= 1` |
| `7-10m` | 第一个小阻塞与恢复 | 卡住是坏了，还是游戏的一部分？ | repair / fallback / reprioritize | blocker taxonomy + recovery CTA；可选动作至少覆盖继续等待、修复阻塞、改道到次优动作之一 | 把挫折变成可处理挑战 | 无恢复路径或无升级解释，标记 `agency_weakened` | `runtime_engineer` + `viewer_engineer` + `qa_engineer` | 10m 卡片记录 `stall_or_wait_periods` |
| `8-12m` | 首个可见产出 / 小胜利 | 我得到了什么？ | 确认产出、领取/查看 | 显示投入、产出、新用途、下一步价值 | 首个“我做成了”的动机点 | 只有库存数字上涨，没有新用途或下一步价值，标记 attraction weak | `gameplay_designer` + `viewer_engineer` | `reward_or_unlock_count >= 1` |
| `12-18m` | 一次性产出转持续能力 | 这不是偶然成功吧？ | 稳定产线、补料、排程 | progress / factory / recipe / recovery 状态持续可读 | 从一次胜利变成能力感 | 进度推进但没有新选择，标记 `progression_pass_but_attraction_weak` | `runtime_engineer` + `gameplay_designer` + `qa_engineer` | capability 样本不只看 `progress_percent` |
| `18-23m` | 第一次扩产取舍 | 我接下来想变强的方向是什么？ | expand / repair / specialize / tradeoff | branch / choice strip 显示扩产、修复、专业化的不同收益，不做营销横幅 | 形成 replay intent | 只有“继续刷同一循环”，标记 `grind_only` | `gameplay_designer` + `producer_system_designer` + `runtime_engineer` | `branch_offer_clarity`、`continue_reason` |
| `20-25m` | 能动性校正 | 我能打断 AI 或重排优先级吗？ | interrupt / reprioritize / correct | `Agency Moves` 暴露打断、重排、纠偏、handoff | 间接控制仍像控制 | 只能等 AI 自己继续，判定 agency 风险 | `agent_engineer` + `viewer_engineer` + `runtime_engineer` | `can_interrupt` / `can_reprioritize` 可读 |
| `25-30m` | 回访理由与小玩家价值 | 我为什么明天还回来？ | 选择下一目标、保存/分享短故事 | `First Win & Anti-Grind`、`Mature-World Continuation`、`Share Replay` | 新 leverage：恢复力、局部用途、议价位或新选择 | `forced major power` / `world_activity_only` 不能算 pass，路由 `PRD-GAME-015` | `gameplay_designer` + `qa_engineer` + `producer_system_designer` | `return_hook`、`leverage_class`、`world_activity_only=no` |

### Content-Volume Supplement / `content_volume_pass` 实现包

当前 TASK-GAME-076 自动化已经能证明 10 个目标 beat 可逐项跑通；新增 scenario driver content-volume supplement 后，required tier summary 已记录 `content_volume_card.status=content_volume_pass`、`effective_play_minutes=34/30`、`player_operation_count=22/18`。下面是已落到 deterministic-provider-backed scenario evidence 的 6 个内容段；它们支撑 design/required gate，不替代真实玩家留存或生产 provider playtest。

| 追加时间 | 新小环节 | 玩家问题 | 玩家动词 | 系统反馈 / surface | 奖励与动机 | 失败 / 恢复 | 需要的 gameplay truth | QA / 自动化验收 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `6-8m` | 任务诊断校准 | 我现在卡在哪，先动哪一步最划算？ | choose diagnosis focus: resource / risk / output | `Diagnosis Focus` 立刻显示瓶颈原因、影响范围、推荐下一步 | 玩家不是盲点按钮，而是在掌控系统 | 选错焦点给低收益诊断，但返还更明确推荐 | `diagnosis_choices[]`、`selected_diagnosis`、`diagnosis_expected_benefit`、`diagnosis_risk` | 点击诊断焦点后 `selected_diagnosis` 存在，`recommendedActions` 更新，`effectivePlayMinutes >= 21` |
| `10-13m` | 资源换路线 / 路线承诺 | 我要快产出，还是稳风险？ | choose route: accelerate / stabilize | `Route Tradeoff` 更新产出预测、风险条、稳定加成，并明确后续会影响委托成本、事故风险、回访目标；若可回退，必须说明回退窗口、代价、保留收益和失去收益 | 第一次轻量 build choice，形成 ownership，并让选择跨 beat 生效 | 风险过高触发 warning，不失败，提供回退到稳态；回退不能只显示布尔值 | `route_tradeoff.mode`、`route_commitment_id`、`output_delta`、`risk_delta`、`stability_delta`、`affected_future_beats[]`、`next_commission_modifier`、`incident_risk_modifier`、`rollback_available`、`rollback_deadline_beat`、`rollback_cost_summary`、`rollback_kept_benefit`、`rollback_lost_benefit` | 点击路线取舍后，后续至少 2 个 beat 的 `micro_commission` / `incident_recovery` / `return_package` 受影响；否则标记 `route_tradeoff_fake_choice`；若 `rollback_available=true` 但缺少窗口/代价说明，标记 `route_rollback_quote_missing` |
| `13-16m` | 微型生产委托 / 可截图成果 | 我能不能让 agent 帮我完成一个小目标？ | issue micro commission: collect / organize / convert | `Micro Commission` 显示 agent 执行步骤、消耗、完成物，并生成成果卡 | 第一个可命名小成果，如 `Starter Batch: Alloy Plate x3`，可交付到本地需求 | 条件不足进入“补材料”恢复动作 | `micro_commission.status`、`requirement`、`steps[]`、`reward_id`、`output_item_id`、`output_display_name`、`batch_size`、`assigned_local_demand_id`、`screenshot_caption`、`delivery_status` | 完成委托后 summary/DOM 同时包含成果名、需求方、交付状态；UI-click flow 要能触达成果卡或交付 CTA |
| `16-19m` | 小事故修复 / 路线后果兑现 | 系统出问题了，我能救回来吗？ | choose repair: quick patch / root-cause fix | `Incident Recovery` 显示事故原因、修复进度、残留风险，并说明它如何来自前一条路线选择 | 失败不是死路，而是玩法；路线风险第一次被兑现 | 快速补丁保进度但留风险；根因排查慢一点但解锁稳定加成 | `incident_recovery.incident_id`、`cause`、`triggered_by_route`、`route_consequence_text`、`repair_options[]`、`selected_repair`、`residual_risk`、`repaired_capability_delta`、`residual_risk_after_repair` | `accelerate` 和 `stabilize` 两条路径的 `route_consequence_text` / `repair_options` / `residual_risk` 必须不同 |
| `21-24m` | 邻近机会探测 / 本局选择生成机会 | 下一块可扩展内容是什么？ | scan direction: market / facility / companion | `Opportunity Scan` 出现 2 个未来 hook，其中 1 个可立即推进，并说明来源于 route / output / repair choice；被推迟/丢弃的 hook 也必须说明价值、未就绪原因、解锁前提和回访时机 | 把 30 分钟后的目标提前种下，同时证明本局选择会生成机会；玩家能理解自己是在取舍而不是被系统单向路由 | 收益低时给情报碎片并推荐另一路；被推迟 hook 不能只消失或只给内部 reason | `opportunity_scan.direction`、`generated_from[]`、`hooks[]`、`immediate_action_id`、`recommended_next_action_reason`、`discarded_hook_reason`、`hook_value_summary`、`hook_readiness_state`、`defer_reason`、`unlock_precondition`、`revisit_timing_hint`、`value_vs_immediate_action` | `generated_from` 至少引用 `route_commitment_id` / `output_item_id` / `selected_repair` 之一；`immediate_action_id` 不能是静态常量；若存在未推荐 hook 但缺少推迟/丢弃取舍说明，标记 `opportunity_discard_reason_missing` |
| `26-30m` | 回访封装 / 第二局差异承诺 | 我今天带走了什么，下次回来干嘛？ | choose achievement card + next-session goal | `Return Package` 生成今日成果、未解锁目标、下次第一步，并引用本局选择记忆 | 明确 small player value：成果、身份、下次动机；第二局不是同一个模板 | 进度不足时生成 recovery plan，而不是空结算 | `return_package.earned_summary`、`choice_memory[]`、`next_session_goal`、`first_action_on_return`、`unlocked_variant`、`why_this_goal`、`recovery_plan` | 至少两条路线产生不同 `next_session_goal` 和 `first_action_on_return`；summary writer 必须报告 `choice_memory` / route branch regression |

#### 最小 gameplay truth 字段

- `onboarding_content_volume.effective_play_minutes`
- `onboarding_content_volume.player_operation_count`
- `onboarding_content_volume.completed_content_beats[]`
- `diagnosis_choices[]`: `id`、`label`、`target`、`expected_benefit`、`risk`
- `selected_diagnosis`
- `route_tradeoff`: `mode`、`route_commitment_id`、`output_delta`、`risk_delta`、`stability_delta`、`affected_future_beats[]`、`next_commission_modifier`、`incident_risk_modifier`、`rollback_available`、`rollback_deadline_beat`、`rollback_cost_summary`、`rollback_kept_benefit`、`rollback_lost_benefit`
- `micro_commission`: `status`、`requirement`、`steps[]`、`reward_id`、`output_item_id`、`output_display_name`、`batch_size`、`assigned_local_demand_id`、`screenshot_caption`、`delivery_status`
- `incident_recovery`: `incident_id`、`cause`、`triggered_by_route`、`route_consequence_text`、`repair_options[]`、`selected_repair`、`residual_risk`、`repaired_capability_delta`、`residual_risk_after_repair`
- `opportunity_scan`: `direction`、`generated_from[]`、`hooks[]`、`immediate_action_id`、`recommended_next_action_reason`、`discarded_hook_reason`、`hook_value_summary`、`hook_readiness_state`、`defer_reason`、`unlock_precondition`、`revisit_timing_hint`、`value_vs_immediate_action`
- `return_package`: `earned_summary`、`choice_memory[]`、`next_session_goal`、`first_action_on_return`、`unlocked_variant`、`why_this_goal`
- `content_volume_card.status`: `content_volume_pass | content_volume_weak`
- `content_volume_card.missing_reasons[]`

#### Viewer 表达要求

- 每个小环节显示一个主问题，例如“先诊断哪里？”而不是只显示系统说明。
- 每次最多 `2-3` 个选项，避免第一小时 UI 过载。
- 每个选项都展示一行预测：`产出 +x / 风险 +y / 稳定 +z`。
- 每次操作后立刻在 `Attraction Proof` 或等价区域显示“你刚刚造成了什么变化”。
- 失败/不足状态不能只报错，必须出现 recovery action。
- `26-30m` 的回访封装要形成可截图/可复盘成果卡：今日成果、未解锁目标、下次第一步。

#### 玩家代理评审后的优化原则

- 目标不是继续堆分钟数，而是把 `content_volume_pass` 从“可跑完的 30 分钟”升级为“玩家能感觉自己的选择改变了后续局面”。
- `route_tradeoff.mode` 不能只改即时数值，必须改变后续至少 2 个 beat，例如委托成本、事故风险或回访目标。
- `route_tradeoff.rollback_available` 不能只作为布尔值；当路线仍可回退时，玩家必须知道回退截止 beat、回退成本、保留收益和失去收益。
- `micro_commission.reward_id` 不能只是 ID，必须形成玩家可见、可截图、可交付的成果卡。
- `local_demand_id` / `contribution_target_id` 不能只证明字段存在，必须出现在成果交付和回访总结中。
- `opportunity_scan` 不能只把一个 hook 推荐为立即动作；被推迟/丢弃的 hook 必须回答“为什么不是现在、缺什么、什么时候回来、相对立即动作差在哪里”。
- `world_change_due_to_player` 必须引用玩家具体选择和具体后果，不能是泛化“世界发生变化”文案。
- `next_session_goal` 不能固定模板，必须由本局路线、成果或修复选择派生。

优化后新增自动化断言：
- `second_run_design_card.status=second_run_hook_pass`
- `route_branch_regression.status=pass`
- `route_tradeoff.affected_future_beats.length >= 2`
- `route_tradeoff.rollback_available=false` 或同时存在 `rollback_deadline_beat`、`rollback_cost_summary`、`rollback_kept_benefit`、`rollback_lost_benefit`
- `micro_commission.output_display_name`、`assigned_local_demand_id`、`screenshot_caption`、`delivery_status != unassigned`
- `opportunity_scan.generated_from[]` 至少引用本局选择之一
- `opportunity_scan.hooks[]` 若包含未选/未推荐 hook，必须有 `hook_value_summary`、`hook_readiness_state`、`defer_reason`、`unlock_precondition`、`revisit_timing_hint`
- `return_package.choice_memory[]`、`unlocked_variant`、`why_this_goal`
- `accelerate` 与 `stabilize` 两条路径的 `next_session_goal`、`first_action_on_return`、`incident_recovery.route_consequence_text` 必须不同

#### 第二轮玩家代理评审后的 anti-script 优化

玩家代理复评结论是“有吸引力但仍偏脚本”：当前流程有吸引力地板，但仍可能让玩家觉得自己只是沿着清晰 CTA 推进。第二轮优化目标不是增加分钟数或内容段，而是在已有 6 个段落中制造中途可见压力、交付进度、第二局记忆、无聊负例和修复代价。

必须落地的 anti-script 断言：
- `anti_script_design_card.status=anti_script_pass`
- `route_tradeoff.midrun_feedback.timing=before_incident_recovery`
- `accelerate` 与 `stabilize` 的 `midrun_feedback.visible_metric_delta`、`local_demand_id`、`incident_recovery.route_consequence_text`、`return_package.next_session_goal` 必须不同
- `micro_commission.assigned_local_demand_id` 必须等于后续 `local_demand_progress_after_delivery.local_demand_id`
- `local_demand_progress_after_delivery.progress_delta > 0`，且 `visible_result_text` 必须显示 before/after/target，例如 `0/6 -> 3/6`
- `return_package.second_session_first_screen_memory` 必须引用本局路线、成果名和下次第一步；两条路线的第二局首屏记忆不得是固定模板
- `boredom_negative_regression.status=pass`，连续 `step / wait / refresh` 且无新 `world_change_due_to_player`、无选择记忆、无奖励时必须检测为 `attraction_weak`
- `incident_recovery.repair_tradeoff_costs.quick_patch.time_cost < root_cause_fix.time_cost`
- `quick_patch.risk_after != root_cause_fix.risk_after`
- `quick_patch.output_delta > root_cause_fix.output_delta` 或 `quick_patch.progress_preserved=true`
- `root_cause_fix.stability_delta > quick_patch.stability_delta`
- 两个 repair option 的 `visible_tradeoff_text` 必须分别说明“现在保进度/未来留风险”和“现在少产出/未来降风险”

#### 调研 / Review Notes

- Onboarding / FTUE 不是把说明塞给玩家，而是让玩家尽早理解核心 loop、目标、反馈和进度；因此补包采用“主问题 + 2-3 个可选动作 + 立即回执”的结构，而不是新增一组说明页。
- 首个 30 分钟需要反复出现 `intent -> action -> consequence -> next choice`，否则会退化成看 agent 自动运行；因此每个新增段都必须有玩家选择或状态改变，防作弊验收禁止 idle/wait 凑时长。
- 玩家留存不只来自理性奖励，也来自“我造成了变化”“我救回了失败”“我有下次目标”的情绪记忆；因此补包保留小事故修复和回访封装，不只补生产效率。
- 隐式学习比硬教程更适合当前工业引导：通过诊断、取舍、委托、修复、探测逐步引入系统，而不是一次性展示所有工业规则。
- 多人大世界/战略游戏参考：
  - EVE Online 的官方 Academy 把新玩家按职业线切入，工业线强调“采集并利用宇宙的 building blocks，塑造 ships / weapons / cities”，适合借鉴为 oasis7 的 `diagnosis -> route tradeoff -> micro commission`，但不能在前 30 分钟灌入完整工业经济。
  - EVE Corporation Projects 把大组织目标拆成可执行项目，适合借鉴为 `return_package.next_session_goal` 和 `completed_content_beats[]`，让小玩家也能看到自己对大世界目标的贡献。
  - Foxhole 后勤资料强调 logistics / facilities / supply line 作为多人战争核心，适合借鉴为 `Input Shortfall`、`Micro Commission` 和 `Local Order`：玩家不是直接看宏大战争，而是先完成一个可验证的补给/转化/交付动作。
  - Albion Online 的经济资料强调本地经济、非全局市场和交易移动，适合借鉴为 `Local Order` / `Opportunity Scan`：让产出有“地点与需求方”，避免库存数字空转。
  - Stellaris beginner materials 把新手引导拆到探索、经济、UI、资源和早期扩张，适合借鉴为 `Opportunity Scan`：给玩家 2 个未来 hook，其中 1 个能马上推进，而不是一次性展开整个战略地图。
- Review risk: 六段新增内容会增加 UI 密度和 runtime 字段面。实现时应先做最小可测结构，避免一次性引入复杂经济系统；每段最多 2-3 个选择，并把 forecast / consequence / recovery 固定成可复用卡片。
- Review risk: 如果微型委托由 agent 自动完成太多，玩家会重新变成旁观者。验收必须检查 `player_operation_count` 和 `world_change_due_to_player`，不能只看 agent activity。
- Review risk: 若路线取舍只有数值文本没有真实 consequence，会变成假选择。QA 必须验证 `route_tradeoff.mode` 会改变后续推荐动作、风险或稳定状态。

设计收敛原则：
- 借鉴对象是“多人长期世界的可感知局部切片”，不是在前 30 分钟引入完整大地图、联盟、战争或市场模拟。
- 每个大世界信号必须压缩成玩家可点击的一步：`诊断短缺`、`选择路线`、`接取本地订单`、`修复事故`、`扫描机会`、`封装回访目标`。
- 每一步必须让玩家看到三个结果之一：库存/产出变化、风险/稳定变化、下一次协作目标变化；不能只出现 lore 文案或背景世界活动。
- 第一局只暴露局部需求方和局部后果，例如“边缘哨站缺板材”“本地订单需求上升”，不暴露全服行情、势力版图或多跳供应链管理。
- 多人感先用“我对一个共享项目有贡献”建立，而不是要求真实多人在线同屏；实现层可以先用 deterministic provider / scenario driver 验证贡献链，后续再接真实多人态。
- QA 验收必须把这些收敛原则转成字段断言：`local_demand_id`、`contribution_target_id`、`world_change_due_to_player`、`next_session_goal`、`recovery_action_id` 至少覆盖新增内容段中的 4 类。

参考来源：
- EVE Academy / Industrialist: https://www.eveonline.com/eve-academy/careers/industrialist
- EVE Academy / Corporation Projects: https://www.eveonline.com/eve-academy/corporation-projects
- Foxhole Official Wiki / Logistics: https://foxhole.wiki.gg/wiki/Community_Guides/Logistics
- Foxhole Official Wiki / Logistics Quickstart: https://foxhole.wiki.gg/wiki/Community_Guides/Logistics_Quickstart
- Albion Online Wiki / Economy: https://wiki.albiononline.com/wiki/Category%3AEconomy
- Albion Online Wiki / Local Economies: https://wiki.albiononline.com/wiki/Local_Economies
- Stellaris Wiki / Beginner's guide: https://stellaris.paradoxwikis.com/Beginner%27s_guide

#### 内容量补足后的验收口径

- `content_volume_card` 已在 deterministic-provider-backed scenario evidence 中达到 pass：`effective_play_minutes=34`、`player_operation_count=22`、`content_unit_count=21`、`distinct_action_family_count=19`。
- 最低新增量曾为 `effective_play_minutes +11`、`player_operation_count +5`、`content_unit_count +4`、`distinct_action_family_count +3`；当前实现超过推荐目标 `effective_play_minutes +14`、`player_operation_count +8`，留有测试误差。
- 新内容必须增加“选择密度”，不能靠等待、长动画、长文案或自动 agent 行为凑时长。
- 若实现时只能先落一半，应保持 `content_volume_weak`，但 summary 要列出剩余缺口，例如 `missing: effective_play_minutes` 或 `player_operation_count`。
- QA 自动化应补两条路径：一条“稳健恢复路线”（诊断 -> 稳态路线 -> 微型委托 -> 根因修复 -> 机会探测 -> 回访封装），一条“高风险/资源不足路线”（加速路线 -> 条件不足 -> 快速补丁 -> 低收益扫描 -> recovery plan）。
- 防作弊验收：每个新增分钟段必须至少有一次玩家选择或状态改变，不能通过 idle/wait 增加分钟数。

### 自动化覆盖矩阵

`TASK-GAME-076` 不允许只靠人工试玩收口。每个 beat 都必须至少有一条 deterministic 自动化断言；人工 attraction card 只能解释“好不好玩”和 boredom reason，不能替代自动化证明。

| 时间 | 必需自动化 surface | 自动化断言 | 不足时的状态 |
| --- | --- | --- | --- |
| `0-1m` | Playwright/headed UI + `software_safe` UI test | 首屏可见 identity / goal / blocker / next；debug/diagnostic surface 不抢主路径焦点 | `goal_clarity_unverified` |
| `1-3m` | Playwright action flow + pure API / runtime contract | 提交 `play` / `step` / gameplay action 后，`accepted_intent`、`execution_state`、`Control Proof` 同步变化 | `first_control_success_unverified` |
| `3-5m` | pure API / runtime event assertion + viewer feedback contract | 同一玩家动作能对账 `player_action` 与 `world_change_due_to_player`；UI 显示 `What I caused` | `world_activity_only_risk` |
| `5-7m` | runtime action catalog test + Playwright button/CTA assertion | 推荐动作有 label、cost/blocker/expected result、可执行或明确 disabled reason | `meaningful_decision_unverified` |
| `7-10m` | runtime blocker/recovery test + Playwright blocked-state assertion | blocker taxonomy、fallback/recovery CTA、next step 同时存在；blocked 不被压成 timeout | `recovery_path_unverified` |
| `8-12m` | pure API / runtime progression test + feedback contract | 首个产出能给出投入、产出、新用途或下一步价值；`reward_or_unlock_count` 可采样 | `reward_or_unlock_unverified` |
| `12-18m` | runtime capability persistence/regression | 产线/能力不是一次性 delta；重新 snapshot 后仍可读 progress / factory / recipe / recovery 状态 | `capability_persistence_unverified` |
| `18-23m` | runtime branch-choice test + Playwright branch strip assertion | 至少出现扩产 / 修复 / 专业化 / 改道之一，且不是同一循环重复刷数值 | `branch_offer_unverified` |
| `20-25m` | agent/runtime command contract + Playwright `Agency Moves` assertion | `can_interrupt` / `can_reprioritize` / correction / handoff 至少有 truth 或明确 unavailable reason | `agency_move_unverified` |
| `25-30m` | feedback contract + Playwright summary assertion + optional Bevy/pixel-world visual check | `First Win & Anti-Grind`、`Mature-World Continuation`、`Share Replay` 可读；若涉及空间/视觉选择，再用 Bevy/pixel-world 验证可见层 | `return_hook_unverified` |

自动化分层口径：
- Playwright / agent-browser：验证真实浏览器里的玩家路径、可见 UI、按钮/CTA、截图与 state，不证明底层世界规则正确。
- Actual UI-click Playwright / agent-browser：验证真实浏览器通过 player-visible controls 点击选择目标、刷新快照、推进一步和推荐动作；`window.__AW_TEST__` 只允许做 readiness、state assertion 与 artifact capture，不允许作为推进玩法的 action mechanism。
- `__AW_TEST__` completeness Playwright / agent-browser：验证测试控制面自身完备，包括 API discovery、control examples、state read、select/focus、snapshot action、step、runSteps、recommended action 和 per-step artifact；这是测试控制面的证据，不替代真实 UI 点击证据。
- `software_safe` contract / Vitest：验证 viewer 派生语义和 DOM 可达性，适合快速挡住 `attractionProof`、`Control Proof`、`Agency Moves` 回退。
- pure API / Rust runtime harness：验证 canonical `player_gameplay`、动作结果、blocker、branch、恢复路径和持久能力；这是非视觉 gameplay truth 的主证据。
- Bevy / pixel-world automation：只用于空间、选择、高亮、可读性、渲染层与 canvas/visual hierarchy；不得单独替代 action -> world change -> next step 的玩法因果测试。

### Scenario Driver / Mock 边界

需要新增共享的 `TASK-GAME-076` scenario driver，而不是让 viewer、Bevy、runtime 各自维护一套 mock 数据。目标是让同一段 0-30 分钟 beat 脚本能同时驱动游戏真值和可视化真值，支持 required 层快速回归，也支持 live 层对账。

设计原则：
- Source of truth 必须是 runtime / pure API 可解释的 canonical scenario：`initial_world -> player_action -> tick/step -> player_gameplay snapshot -> render_state`。
- Viewer fixture 只能由 canonical scenario 导出的 `snapshot.player_gameplay` / model / feedback 生成，不允许手写第二套 `attractionProof` 结论。
- Bevy / pixel-world fixture 只能由同一 scenario 的 model/selection/render input 派生，不允许用视觉 mock 反推玩法 pass。
- Agent 可使用 deterministic provider/mock provider 作为测试驱动，但必须显式标记 `provider_mode=mock` / `deterministic`，不得把它包装成 active-LLM fresh sample。
- required 层允许用 scenario driver 生成 deterministic fixtures；live 层必须用真实 stack / pure API / browser 操作复跑同一 scenario，并比较关键字段。

最小机制：
1. `scenario.json` / Rust fixture builder 描述每个 beat 的输入动作、期望 canonical 字段、期望 UI/visual 字段和允许缺口状态。
2. Rust runtime harness 执行 scenario，产出 `player_gameplay` snapshots 与 event/step 证据。
3. Viewer test/API 读取同一 snapshot，验证 `Control Proof`、`Attraction Proof`、`Agency Moves` 等派生 UI。
4. Bevy/pixel-world 读取同一 render input，验证空间/高亮/可读层。
5. Summary 必须标明每个 beat 是 `runtime_backed`、`viewer_fixture_only`、`visual_only`、`live_verified` 或 `unverified`。

### Truth / Surface 边界
- 应复用或补齐 canonical `player_gameplay` / feedback 真值：`accepted_intent_id`、`intent_summary`、`execution_state`、`status_reason`、`goal_title`、`progress_percent`、`blocker_kind`、`next_step_hint`、`fallback_action_*`、`available_actions`、`player_action`、`world_change_due_to_player`、`leverage_class`、`same_loop_repeat_count`、`grind_only_flag`、`major_power_dependency_status`、`repair_available`、`rebuild_available`、`pivot_available`。
- Viewer / pure API 只表达这些真值，不私算第二套 gameplay truth；缺字段时显示 `unverified` / `missing truth` / `not published`，不得伪造按钮或奖励。
- `hook_score`、`replay_intent`、`meaningful_decision_count`、`reward_or_unlock_count`、`stall_or_wait_periods`、`biggest_boredom_point`、`continue_reason` 属于 TASK-GAME-076 attraction card / QA evidence DTO，不应写进 runtime snapshot 当作世界真值。

### Agent Feedback Contract
- 每个 agent 行动回执至少要能形成玩家语义链：`intent_summary -> decision_reason -> execution_status -> world_change_summary -> next_step_hint`。
- 关键字段包括：`decision_reason`、`primary_reason`、`cost_summary`、`progress_delta`、`reward_or_unlock`、`blocker_type`、`fallback_action`、`reprioritize_options`、`resume_anchor`。
- agent 不能无声替玩家长期改道；阻塞后最多给推荐和可执行 action/template，玩家仍要能 interrupt / reprioritize / correct。
- 若推荐不可执行，必须给 `why_not_executable`；若 context 过期，必须暴露 stale / missing prerequisite，而不是继续给泛化建议。

### 实现包拆分
1. `runtime_engineer` + `agent_engineer`: canonical decision receipt
   - 固定 `accepted_intent -> decision_reason -> execution_status -> world_change -> next_step` 的结构化真值。
   - 验收关注 rejected / blocked / completed_no_progress / completed_with_progress 都有主因果和 fallback。
2. `viewer_engineer`: player-facing proof chips
   - 在现有 `Formal Gameplay Summary` 容器下优先加厚 `Control Proof`、`Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation`。
   - proof chips 使用短标签：`What I caused`、`New option`、`Why continue`、`Waiting cost`、`Recovery`。
3. `gameplay_designer` + `qa_engineer`: attraction evidence
   - 3 个 deterministic-provider-backed attraction cards；fresh active-LLM / headed UI cards 仍可作为 release/playtest 补证。
   - 1 组 30-minute motivation-density card，写入 automation summary 的 `attraction_evidence`。
   - 失败 taxonomy 至少包含 `silent_agent`、`non_executable_suggestion`、`world_activity_only`、`no_new_choice`、`agent_autoplays_too_much`、`grind_only`。
4. `producer_system_designer`: scope裁决
   - 若 `progression_pass_but_attraction_weak` 出现，优先补 beat 级 motivation density，不扩大宏系统卖点。
   - 只有 `TASK-GAME-076` 样本证明 attraction pass 后，才评估是否恢复 preview 节奏或扩展后续内容。

### 验证卡字段
- 10 分钟 attraction card: `hook_score`、`replay_intent`、`action_effect_feedback`、`what_did_i_cause`、`biggest_boredom_point`、`no_op_or_follow_up_route`。
- 30 分钟 motivation-density card: `meaningful_decision_count`、`reward_or_unlock_count`、`stall_or_wait_periods`、`branch_offer_clarity`、`continue_reason`、`return_hook`、`leverage_class`。
- 30 分钟 content-volume card: `effective_play_minutes`、`target_effective_play_minutes`、`player_operation_count`、`content_unit_count`、`distinct_action_family_count`、`passive_wait_minutes`、`passive_wait_share`；当前 TASK-GAME-076 deterministic evidence 覆盖 `34/30` 分钟有效内容，应判 `content_volume_pass`。
- weak-sample regression: `weak_high_progress` 必须识别为 `progression_pass_but_attraction_weak`，不得进入 `attraction_pass`。
- required 证据：`software_safe` / headed UI 截图覆盖 empty、accepted、executing、blocked、completed、branch-ready、grind-only、recovery-ready；pure API 对账 accepted intent / status / blocker / next step。
- suggested commands:
  - `rtk node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
  - `rtk node crates/oasis7_viewer/scripts/gameplay-attraction-scenario.test.mjs`
  - `rtk node crates/oasis7_viewer/scripts/gameplay-attraction-summary-writer.test.mjs`
  - `rtk npm run test:ui -- software_safe_src/main.test.jsx`
  - `rtk ./scripts/verify-gameplay-attraction-automation.sh --tier required`
  - `rtk ./scripts/verify-gameplay-attraction-automation.sh --tier live`
  - `rtk ./scripts/viewer-gameplay-attraction-playthrough.sh ...` 用于单独调试逐项 Playwright 30 分钟脚本；正式矩阵仍由 `--tier live` 统一调用。
  - `rtk ./scripts/viewer-gameplay-attraction-ui-click-playthrough.sh ...` 用于单独调试实际 UI 点击版 30 分钟脚本；正式矩阵中对应 `live_browser_30m_ui_click_playthrough`。
  - `rtk ./scripts/viewer-aw-test-completeness-playthrough.sh ...` 用于单独调试 `__AW_TEST__` 完备性脚本；正式矩阵中对应 `live_aw_test_completeness_playthrough`。
  - active-LLM / headed UI fresh sample 采集仍可作为 release/playtest 补证，证据回写到 `doc/playability_test_result/` 或 `doc/testing/evidence/`。

## 风险
- 若 `TASK-GAME-062` 未先收口，`TASK-GAME-063/064` 的任何正向结论都可能是脆弱假象。
- 若 `TASK-GAME-065` 不区分 active-LLM 与 debug lane，门禁会再次失真。
- 若 `TASK-GAME-076` 只补 UI 文案而不补决策密度，结果会变成“解释更清楚但仍不好玩”。
- 若 agent 自动替玩家完成改道、恢复和长期目标选择，会把间接控制重新退化成旁观。
