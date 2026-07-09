# Gameplay 小玩家成长线与成熟世界承接（2026-05-17）设计文档

- 对应需求文档: `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.project.md`

审计轮次: 1

## 设计目标
- 给 mature world state 下一条“小玩家还能继续玩什么”的正式玩法答案。
- 明确该答案如何承接现有 `PostOnboarding`、`slot-1` claim、first capability、player leverage 与 control-feeling 合同。
- 把“受保护 first win、专业化、小范围影响力、恢复路径”拆成 runtime / viewer / agent / QA 共同实现的合同，而不是一段愿景口号。

## Canonical Lane

### 1. Entry Gate
- 该线不属于首个 10 分钟 onboarding。
- canonical entry gate 是：
  - `PostOnboarding` 已进入正式阶段；
  - `first capability gate` 已达成或至少已有明确 canonical entry；
  - 玩家拥有可操作的 `slot-1` claim / starter funding / equivalent local foothold。

### 2. Default Mainline: Local Operator
- 第 1 条正式 small-player 主线不是“立刻建联盟”或“立刻进全球治理”。
- 默认主线是 `local operator`：
  - 在一个有限区域内把 1 条小规模工业能力稳定下来；
  - 产出至少 1 次对世界有可见后果的结果；
  - 让玩家能清楚回答“我刚刚让这个地方变得更好了什么”。

### 3. Protected First Win
- `protected first industrial win` 的保护含义不等于免战、免竞争、免治理。
- 这里的“保护”拆成 3 层：
  - `low strategic footprint`: 早期小玩家的首个胜利体量较小，不应一上来就与 major-power 主战略面重叠。
  - `bounded downside`: 失败时不应直接把账号打回“只能重开号或只能投靠大组织”的死局。
  - `recoverable path`: 玩家必须拥有 repair / rebuild / pivot 的显式下一步。

### 4. Specialization After Stability
- `local operator` 不该无限重复同一件事。
- 在完成首个稳定胜利后，小玩家应切到至少 1 条专业化方向。当前设计冻结 3 类候选：
  - `recovery-operator`: 擅长恢复停机、补齐缺口、保持局部韧性。
  - `conversion-specialist`: 把局部丰富原料转成区域更紧缺的中间品或制成品。
  - `regional-service-runner`: 提供局部 upkeep / 补给 / 维护类服务，形成稳定 return hook。
- 专业化选择前必须展示 `specialization_entry_quote` / `first_delivery_preview`：
  - `specialization_id`
  - `target_local_demand_id`
  - `first_output_preview`
  - `estimated_delivery_ticks`
  - `required_inputs`
  - `switch_cost_class`
  - `leverage_class_unlocked`
  - `regional_usefulness_reason`
  - `return_hook_after_delivery`
- 若专业化推荐缺少第一单本地需求、交付产出或 leverage 解锁说明，标记 `specialization_delivery_preview_missing`；这表示选择可读性不足，不代表要新增完整职业系统。

### 5. Limited-Scope Regional Influence
- 小玩家路线必须在 mature world 中拥有“局部有用”的后果，而不是只能赚钱或存活。
- 但这层后果必须小于 global governance / alliance leadership：
  - 更接近 `regional priority / local opportunity / local visibility / local trust`；
  - 不直接等价为“全局投票权、主链治理主导、跨区军政控制”。

### 6. Recoverable Failure
- small-player lane 的失败不允许默认收束为“只能归顺大组织”。
- 当前要求至少提供 3 类恢复语义：
  - `repair`: 维持同一路线，修复停机或缺口。
  - `rebuild`: 更换据点或重新建立同类小规模能力。
  - `pivot`: 改做另一条专业化角色，而不是继续硬顶当前失败链。

### 7. Agent Specialization Contract
- Agent 行为合同优先作为文档合同收口；它不先新增 runtime 权限、claim 旁路或 release claim。
- Agent 在 mature-world small-player lane 中消费 `player_gameplay` 已有字段，尤其是：
  - `small_player_lane_id`
  - `leverage_class`
  - `same_loop_repeat_count`
  - `grind_only_flag`
  - `major_power_dependency_status`
  - `recovery_path_kind`
  - `recovery_path_detail`
  - `requires_major_power_sponsorship`
  - `repair_available` / `rebuild_available` / `pivot_available`
- Agent 推荐专业化或恢复时，面向玩家的解释必须回答：
  - 我建议你做什么；
  - 为什么这仍属于独立 small-player lane；
  - 失败代价和恢复范围是什么；
  - 卡住时如何 `repair / rebuild / pivot`；
  - 是否强制依附 major power。
- Agent-facing 摘要后续实现时应暴露：
  - `selected_specialization_id`
  - `specialization_reason`
  - `preferred_next_action_class`
  - `dependency_boundary`
  - `recovery_escalation_reason`

### 8. Agent Decision Order
- 默认顺序：
  1. `local_operator`: 直到首个稳定世界变化可见。
  2. `recovery_operator`: 修复 blocked line、补齐缺口、保持局部能力韧性。
  3. `conversion_specialist`: 把区域富余输入转成区域有用的中间品或制成品。
  4. `regional_service_runner`: 提供 upkeep / supply / repair / logistics 服务，形成短周期 return hook。
- 若 `major_power_dependency_status=independent_path_available` 或 `sponsor_helpful_not_required`，Agent 必须先给本地 `repair / rebuild / pivot`，再考虑 sponsor / alliance / governance。
- 若 `requires_major_power_sponsorship=no`，Agent 不得把 sponsor 或 major-power dependency 写成必需步骤。
- 若 `same_loop_repeat_count >= 3` 且 `leverage_class=throughput_only` 或 `unclassified`，Agent 必须停止强化同一生产循环，转向专业化、恢复或新的区域用途。
- 若 `recovery_path_kind=repair_rebuild_or_pivot`，`wait / wait_ticks` 默认不是有效恢复，除非存在明确 cooldown 或外部依赖。
- major-power escalation 只允许作为 `voluntary_escalation`，或 runtime 已标记 `major_power_dependency_status=forced` 时的有因升级。

### 9. Agent Anti-Drift Checks
- blocker: `requires_major_power_sponsorship=no` 时，Agent 把 join alliance / sponsor / patron / major power 写成必需。
- blocker: `grind_only_flag=true` 后，Agent 继续重复 throughput-only 生产。
- blocker: `local_operator` 后第一专业化直接跳到 global governance、alliance leadership、war 或 major-power membership。
- watch: 可恢复失败或 full recipe coverage 后，Agent 仍输出无原因的 `wait / wait_ticks`。
- watch: `specialization_reason` 缺少 `player_action`、`world_change_due_to_player` 或 `return_hook`。
- watch: `specialization_entry_quote` 缺少 `target_local_demand_id`、`first_output_preview`、`estimated_delivery_ticks` 或 `leverage_class_unlocked`，导致玩家只看到专业化标签。

## 与现有专题的边界
- `PRD-GAME-007`
  - 管 `PostOnboarding` 从 onboarding 过渡到 first capability。
  - 本专题管理 first capability 之后的小玩家承接。
- `PRD-GAME-011`
  - 管 `slot-1` claim、starter funding 与非免费 claim 经济边界。
  - 本专题复用这些 foothold，不重开免费路径。
- `PRD-GAME-012`
  - 管当前 early-retention 主冲刺与 trust/capability verdict。
  - 本专题不改写当前主 blocker 顺序，只定义后续路线。
- `PRD-GAME-014`
  - 管 accepted intent、主因果、打断重排与续玩恢复的 agency 合同。
  - 本专题要求 small-player lane 也必须回答“我做了什么、为什么现在这样、下一步该做什么”。
- `player leverage rubric`
  - 本专题把 `player leverage != world activity` 当成硬门槛，而不是补充说明。

## 角色切片
- `producer_system_designer`
  - 冻结 lane、边界、恢复与区域影响上限。
  - 裁决哪些专业化仍属于“小玩家路线”，哪些已经属于 major-power escalation。
- `runtime_engineer`
  - 把 lane entry、checkpoint、failure signature、recovery option 写成 canonical truth。
- `viewer_engineer`
  - 把 lane、首个胜利、区域价值与恢复路径做成显式 surface。
- `agent_engineer`
  - 对齐 specialization / recovery / org-independence 行为合同，避免默认把玩家推向依附。
  - 后续若进入代码实现，负责把 `selected_specialization_id`、`specialization_reason`、`dependency_boundary` 等字段落到 prompt、trace 或 snapshot。
- `qa_engineer`
  - 建立 mature-world small-player matrix，阻断 `world_activity_only` 误报。

## 实施顺序
1. `small-player-progression-contract-freeze`: 冻结专题并回挂根入口、主文档、索引、execution log。
2. `runtime-small-player-lane-state-contract`: 下沉 lane state / checkpoint / recovery truth。
3. `viewer-small-player-lane-surface-alignment`: 收口 headed Web/UI 与 pure API 承接 surface。
4. `agent-small-player-specialization-contract`: 对齐专业化、恢复与 org-independence contract；文档合同完成后仍需 QA fresh mature-world sample 才能升级 lane verdict。
5. `qa-small-player-progression-matrix`: 建立 mature-world 小玩家矩阵与 blocker 签名。

## 验证口径
- required:
  - 文档互链与任务映射。
  - lane、player leverage、recovery 与 limited-scope influence 边界可 grep、可对账。
  - Agent 合同中的 specialization / recovery / org-independence、decision order 与 anti-drift blocker 可 grep、可对账。
  - headed Web/UI 与 pure API 至少能表达当前小玩家 lane 的主语义。
- full:
  - mature-world playability 样本复核。
  - `player leverage` / `world_activity_only` 抽样。
  - failure -> repair/rebuild/pivot 代表性样本。

## 2026-06-25 Follow-up: P1/P2 Viewer Surface
- `software_safe` 已把本设计合同中的玩家能动性、首个工业胜利、anti-grind leverage 与 repair/rebuild/pivot 承接，呈现为 `Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation` 与 `Share Replay`。
- 该 follow-up 是 headed Web/UI 的呈现层对齐：它可以让制作人快速看到 `player_action`、`world_change_due_to_player`、`leverage_class`、`major_power_dependency_status` 与恢复选项，但不替代 runtime lane truth、pure API parity、agent specialization contract 或 QA mature-world verdict。
- 当字段缺失时，viewer 必须显示 `unverified` / `not_published` / waiting-state，而不是把小玩家 lane 误包装为 pass。

## 风险
- 如果只定义“更多工业”，而不定义区域性 usefulness，小玩家路线会退化成 grind。
- 如果把局部影响力定义得太强，会抢占 mature-world 大组织路线并引发 claim drift。
- 如果不把恢复路径写进合同，任何局部失败都会把玩家再次打回“要么退坑、要么依附”的旧结构。
