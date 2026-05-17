# Gameplay 小玩家成长线与成熟世界承接（2026-05-17） PRD v0.1

- 对应设计文档: `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.project.md`

审计轮次: 1

## 1. Executive Summary

- Problem Statement: `PRD-GAME-007/011/012/014` 已经覆盖“首个 claim、首个持续能力、10 分钟 trust gate 与间接控制 agency”，但当前仓库还缺一份正式玩法合同来回答：当世界已经存在更强组织、既有治理和长期历史时，小玩家/新玩家在不立即投靠大组织的前提下，靠什么继续有独立价值。缺少这份合同，玩家很容易在完成首个工业里程碑后退化为旁观者、打工号，或只能把“大世界很活跃”误报成“我还有 meaningful participation”。
- Proposed Solution: 新增 `PRD-GAME-015`，冻结一条正式 `small-player lane`：`PostOnboarding / first capability` 之后，玩家默认进入“`local operator -> regional specialist -> limited-scope regional influence`”承接线。该线复用现有 `slot-1` claim、starter funding、player leverage rubric 与 control-feeling 合同，把“受保护的首个工业胜利、小规模专业化、有限区域影响力、可恢复失败路径”写成可拆任务、可验收、可回归的玩法真值。
- Success Criteria:
  - SC-1: `game` 根 PRD / project、`gameplay` 主文档和新专题统一采用 `small-player lane` 口径，不再把“完成首个能力”直接等同于“必须马上加入大组织或进入深治理”。
  - SC-2: 至少冻结 1 条可信的小玩家主线，明确 entry gate、阶段检查点、专业化分支、有限区域影响力和恢复路径。
  - SC-3: `protected first industrial win` 被正式定义为“低爆炸半径、可恢复、可见 player leverage”的 first win，而不是“军事无敌新手保护泡泡”。
  - SC-4: 后续 runtime / viewer / agent / QA 至少各有 1 条 follow-up task，可直接验证“玩家做了什么、世界因此改变了什么、下一步为何仍值得继续”。
  - SC-5: 本专题显式保持当前 `internal_playable_alpha_late` 与 `limited playable technical preview` 边界，不把 `#165` 的专题冻结包装成阶段升级或 broader launch 结论。

## 2. User Experience & Functionality

- User Personas:
  - 小玩家 / 新玩家：已经通过首次 onboarding 与首个持续能力，但仍只有 1 个小规模 claim 或有限资源，不想立即依附大组织也希望继续有独立价值。
  - 回流玩家：中断后回到成熟世界，需要一条能重新站稳脚跟的恢复型小玩家路线，而不是只能读 raw log 猜当前局势。
  - `producer_system_designer`: 需要把“小玩家在成熟世界中如何不沦为旁观者”冻结成正式玩法边界，而不是临时说明。
  - `runtime_engineer`: 需要知道哪些状态、阶段和失败签名必须成为 canonical lane truth。
  - `viewer_engineer`: 需要把“小玩家 lane 的主目标、首个胜利、专业化选择、区域影响与恢复路径”做成玩家可见 surface。
  - `agent_engineer`: 需要对齐 agent 在成熟世界下的专业化偏好、恢复优先级和“别自动把玩家推去投靠大组织”的边界。
  - `qa_engineer`: 需要一套矩阵去分辨“玩家真的产生了 leverage”与“世界只是继续在自己运转”。
- User Scenarios & Frequency:
  - 首个持续能力达成后：每个认真进入中循环的玩家至少 1 次。
  - 成熟世界再进入：当世界已有强组织、区域分工和既有历史时，反复发生。
  - 失败恢复：小玩家丢失产线、缺料、被竞争者挤压或 claim 失效时，多次发生。
  - 专业化切换：从“先活下来”切到“区域性有用角色”时至少 1 次，后续可重复。
- User Stories:
  - PRD-GAME-015: As a 小玩家 / 新玩家, I want a meaningful progression lane that stays viable in a mature world without immediate alignment to major powers, so that I can keep building leverage after the first capability milestone.
  - PRD-GAME-015A: As a `producer_system_designer`, I want `protected first win`, `regional usefulness`, and `recoverable failure` formalized, so that mature-world progression stops depending on implicit tribal knowledge.
  - PRD-GAME-015B: As a `qa_engineer`, I want the lane to require explicit player-leverage evidence rather than ambient world activity, so that “the world is busy” cannot be misreported as “the player still has agency”.
- Critical User Flows:
  1. Flow-SPL-001: `玩家完成 PostOnboarding / first capability -> 系统确认 slot-1 claim / starter funding / current lane entry gate -> 正式转入 small-player lane`
  2. Flow-SPL-002: `系统展示默认 local operator 主线 -> 玩家完成一个受保护的首个工业胜利 -> surface 明确回答 player_action / world_change_due_to_player / next_step`
  3. Flow-SPL-003: `玩家从默认主线进入 1 条短周期专业化分支 -> 持续贡献一个区域性可见价值，而不是直接进入全球治理或大规模战争`
  4. Flow-SPL-004: `玩家遭遇缺料、停机、claim 丢失或局势挤压 -> 系统提供恢复路径或低成本改道 -> 不要求立即依附 major power 才能继续`
  5. Flow-SPL-005: `玩家累计区域性 player leverage -> 获得 limited-scope regional influence -> 再决定保持独立专业化、局部合作，或自愿进入更大组织/更深治理`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| small-player lane contract | `lane_id`、`entry_gate`、`requires_major_power_alignment`、`first_win_definition`、`mature_world_value_definition` | 不要求立即加入组织；系统在达成 `first capability` 后显式进入 lane | `not_eligible -> eligible -> active -> specialized -> regionally_useful` | 先完成首个持续能力，再进入小玩家 lane；不得把 onboarding 与 mature-world lane 混写 | `producer_system_designer` 冻结；runtime/viewer/agent/QA 依合同实施 |
| protected first industrial win | `first_win_goal_id`、`player_action`、`world_change_due_to_player`、`player_leverage_verdict`、`blast_radius_class`、`recovery_cost_class` | 玩家完成 1 次低爆炸半径、可恢复、对世界有可见变化的工业胜利 | `not_started -> executing -> first_win_complete -> stabilized` | “受保护”优先指低战略体量、低爆炸半径和可恢复路径，不等于 PVP/治理免疫 | 任何正式放行结论都必须回答 leverage，而不是 world activity |
| specialization pack | `specialization_id`、`input_profile`、`output_profile`、`regional_usefulness`、`switch_cost_class`、`org_independence_level` | 玩家从默认 local operator 主线切到 1 条短周期专业化角色 | `unselected -> selected -> delivering -> stable` | 先 local survival，再 specialization；默认避免一开始把玩家导向深治理/大战争 | `agent_engineer` / `viewer_engineer` 对齐；`producer_system_designer` 裁决专业化边界 |
| limited-scope regional influence | `influence_surface_id`、`influence_scope`、`influence_cap`、`expires_on_inactivity`、`converts_to_global_governance` | 玩家通过区域性持续贡献获得有限影响力或优先级 | `locked -> visible -> earned -> decays_on_inactivity` | 区域影响力必须小于 global governance / alliance leadership；默认随停摆或长期闲置衰减 | 不得直接等价为 global vote power、主链治理权或 major-power membership |
| recovery path | `failure_signature`、`recovery_option_id`、`restoration_scope`、`fallback_specialization_id`、`requires_major_power_sponsorship` | 玩家遭遇 claim/产线/区域性挤压失败时，系统给出恢复或改道选项 | `healthy -> disrupted -> recoverable -> restored/pivoted` | 优先提供低成本修复、局部重建或改道；默认不要求先加入大组织 | `requires_major_power_sponsorship` 默认应为 `no`；只有更高阶路线才允许提升依赖 |
| mature-world guardrails | `world_activity_only`、`major_power_dependency_status`、`regional_pressure_level`、`return_hook` | QA / playability surface 明确区分“世界很活跃”和“玩家仍然有 lane” | `unclear -> bounded -> verified` | 只要 `world_activity_only=yes` 或 `major_power_dependency_status=forced`，该样本就不能支撑 lane `pass` | `qa_engineer` 守门，`producer_system_designer` 最终裁决 |

- Acceptance Criteria:
  - AC-1: 本专题至少定义 1 条正式 `small-player lane`，且该 lane 的 entry gate 明确发生在 `PostOnboarding / first capability` 之后，而不是回退到 onboarding。
  - AC-2: 该 lane 明确写出 `protected first industrial win` 的定义，并说明“保护”指低爆炸半径、可恢复和 player leverage 可见，不指永久免战或特殊政治豁免。
  - AC-3: 玩家可以在不立即加入 major power 的前提下完成 1 次 meaningful outcome；文档必须明确该 outcome 的世界变化、下一步和区域性价值。
  - AC-4: 本专题至少定义 1 条默认主线与 2 条可延展 specialization 方向，并说明它们为何在 mature world 中仍有独立价值。
  - AC-5: 文档必须明确 `limited-scope regional influence` 的边界：它可以改变局部优先级、区域可见度或区域性机会，但不能直接等价为 global governance 权力。
  - AC-6: 文档必须定义 recoverable failure path；当玩家遭遇停机、claim 丢失、局部竞争失败或区域压力时，不得要求“只能投靠大组织”作为唯一继续路径。
  - AC-7: `player leverage != world activity` 的约束必须进入本专题完成定义；任何 lane `pass` 都必须回答 `player_action / world_change_due_to_player / return_hook`。
  - AC-8: 本专题必须显式声明不改变当前 `PRD-GAME-012` 的 early-retention 主优先级，也不把 `#165` 写成当前 stage 或 preview claim envelope 的升级依据。
  - AC-9: `game` 根 PRD / project、`gameplay` 主文档、`README`、`prd.index` 与当前 task execution log 必须能互链到 `PRD-GAME-015`。
  - AC-10: 至少拆出 `producer_system_designer`、`runtime_engineer`、`viewer_engineer`、`agent_engineer`、`qa_engineer` 五类后续任务，并给出 `test_tier_required` / `test_tier_full` 验收方向。
- Non-Goals:
  - 不在本专题里把小玩家路线直接升级为全球治理路线、联盟领袖路线或大战争主线。
  - 不在本专题里承诺新的免费 claim、额外 unrestricted token 补贴或新的 economic bypass。
  - 不把“受保护 first win”写成永久新手护盾、战斗无敌或不可被干预的独占区。
  - 不把 `#165` 当作当前 `limited playable technical preview` 的放大器；它只定义玩法承接，不替代外部真实信号。

## 3. AI System Requirements (If Applicable)

- Tool Requirements:
  - `PostOnboarding` / `player_gameplay` canonical snapshot。
  - `player leverage` / `world_activity_only` 证据字段与 playability card。
  - limited preview / trust gate / capability gate 的现有 formal evidence。
- Evaluation Strategy:
  - 以 `player leverage`、`return_hook`、`major_power_dependency_status` 与 `recoverable failure` 四条线评估，而不是只看“世界是否还在活跃”或“玩家是否已经看到很多系统”。

## 4. Technical Specifications

- Architecture Overview:
  - `PRD-GAME-015` 建立在现有 `PRD-GAME-007` `PostOnboarding` 阶段承接、`PRD-GAME-011` `slot-1` claim / starter funding、`PRD-GAME-012` trust/capability gate、`PRD-GAME-014` control-feeling 合同之上。
  - runtime 负责把 lane entry / checkpoint / failure signature / recovery path 下沉为 canonical truth。
  - viewer / pure API 负责把“你当前属于哪条小玩家 lane、刚刚赢了什么、为什么仍然值得继续”做成显式 surface。
  - agent contract 负责把 specialization / recovery 偏好写进可解释行动面，而不是默认把玩家推向 major-power dependency。
  - QA / playability evidence 负责阻断 `world_activity_only` 误报，并确认这条 lane 在 mature world 中仍成立。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/prd.index.md`
  - `doc/game/README.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-top-level-design.project.md`
  - `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
  - `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
  - `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
  - `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `doc/playability_test_result/prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 区域内已有大型组织垄断：lane 仍需给出局部独立价值和恢复路径，而不是默认判定“新玩家只能加入他们”。
  - 玩家完成 first capability，但 local site 因缺料/停机/战乱无法继续：必须提供恢复或改道选项，不能让 lane 直接失效。
  - 世界很活跃，但玩家没有造成明确世界变化：必须标记 `world_activity_only=yes`，且不得把该样本判为 lane success。
  - 玩家主动加入大型组织：允许，但文档必须说明这属于 voluntary escalation，而不是 lane 的强制前提。
  - 玩家失去首个 claim 或 starter industrial node：必须至少保留一条小规模 rebuild / pivot path，而不是把失败等同于“重新开号”。
  - 区域性影响力被误写成 global governance：判定为 scope drift，必须回退。
- Non-Functional Requirements:
  - NFR-SPL-1: `PRD-GAME-015` 的根入口互链必须在 1 个工作日内完成并可 grep。
  - NFR-SPL-2: 任一正式 small-player lane 样本 100% 必须明确 `player_action`、`world_change_due_to_player`、`return_hook` 与 `world_activity_only`，不得只给“世界有变化”。
  - NFR-SPL-3: 任一正式 lane 不得把“立即加入 major power”作为唯一 entry requirement；若出现该依赖，默认判定为 lane contract 失败。
  - NFR-SPL-4: `limited-scope regional influence` 100% 必须与 global governance / alliance leadership 分开定义，不得偷渡成更强权力口径。
  - NFR-SPL-5: 本专题不得改写当前 `limited playable technical preview` claim envelope，也不得用来替代 `PRD-GAME-012` 的 trust/capability 样本。
- Security & Privacy:
  - 本专题不新增额外账号权限或经济旁路；仍遵循现有 claim / restricted starter / governance 审计边界。

## 5. Risks & Roadmap

- Phased Rollout:
  - R0: 冻结 `PRD-GAME-015`，完成根入口、主文档、索引与 task 映射挂载。
  - R1: runtime 定义 small-player lane 的 checkpoint / failure / recovery canonical truth。
  - R2: viewer / pure API 收口 lane surface，让玩家看懂当前主线、首个胜利、专业化与恢复路径。
  - R3: agent 对齐 specialization / recovery / org-independence contract。
  - R4: QA 建立 mature-world small-player matrix，并与 `player leverage` rubric 联动。
- Technical Risks:
  - 风险-1: 如果 lane 只剩“再做更多工业”，而没有区域性价值与恢复路径，会退化成重复 grind。
  - 风险-2: 如果 lane 一开始就给出过强区域/治理权力，会与 mature-world 大组织路线互相冲突，放大平衡风险。
  - 风险-3: 如果 lane 只在文档里存在、没有 canonical checkpoint 和 player leverage surface，后续仍会回到“世界很热闹但我不确定自己有用”的旧问题。

## 6. Validation & Decision Record

- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-015 | `small-player-progression-contract-freeze` | `test_tier_required` | 文档治理检查、根入口/主文档/索引/execution log 互链核验 | mature-world 小玩家路线合同冻结 |
| PRD-GAME-015 | `runtime-small-player-lane-state-contract` | `test_tier_required` | canonical lane state / checkpoint / failure / recovery truth 对账与定向测试 | runtime / `player_gameplay` 语义承接 |
| PRD-GAME-015 | `viewer-small-player-lane-surface-alignment` | `test_tier_required` | headed Web/UI 与 pure API surface 复核，确认 lane、首个胜利、区域价值与恢复路径可读 | 玩家可见承接、return hook 与多端一致性 |
| PRD-GAME-015 | `agent-small-player-specialization-contract` | `test_tier_required` | specialization / recovery / org-independence contract 对账，确认 agent 不默认把玩家推向强依附 | agent 行为边界、间接控制下的专业化表达 |
| PRD-GAME-015 | `qa-small-player-progression-matrix` | `test_tier_required` + `test_tier_full` | player leverage rubric、mature-world small-player 样本、failure/recovery blocker 签名与 lane verdict 归档 | 玩法承接、mature-world 有效性与误报阻断 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SPL-001 | 新增独立 `PRD-GAME-015`，专门定义 mature-world 小玩家成长线 | 继续把“小玩家如何继续玩”散落在 PostOnboarding、claim economy、playability evidence 与 issue 讨论中 | 当前缺的不是“首个能力能否完成”，而是“完成之后为何仍有独立价值”的正式合同。 |
| DEC-SPL-002 | 把 `small-player lane` 起点放在 `first capability` 之后，而不是重新塞回首个 10 分钟 | 让 issue #165 与当前 early-retention 冲刺混成一个问题 | `PRD-GAME-012` 当前仍是主 blocker；如果把 mature-world 设计提前塞进 first-session，会打乱当前冲刺排序。 |
| DEC-SPL-003 | 把 `protected first industrial win` 定义为“低爆炸半径 + 可恢复 + leverage 可见” | 把“保护”写成 PVP/政治完全免疫，或继续不定义保护含义 | 完全免疫会制造失真预期；不定义又会让“首个胜利”在成熟世界里没有真实站得住脚的边界。 |
| DEC-SPL-004 | 采用“limited-scope regional influence”，明确低于 global governance / alliance leadership | 让小玩家 first-success 直接跳到全局治理权，或反过来完全不给任何区域性影响 | 没有局部影响力，这条线会像重复打工；给太强又会破坏 mature-world 权力结构。 |
| DEC-SPL-005 | 把 `player leverage != world activity` 写成本专题硬门槛 | 继续允许“世界很活跃”替代“玩家仍然有 meaningful participation” | `#165` 真正要解决的是小玩家是否还在推动世界，而不是世界是否本来就有很多事在发生。 |
