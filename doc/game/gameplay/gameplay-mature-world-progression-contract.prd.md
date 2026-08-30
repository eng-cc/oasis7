# 成熟世界小玩家成长合同

- 上层产品映射：本合同承接 `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md` 的成熟世界产品承诺与 `doc/game/prd.md` 的 `PRD-GAME-015`。
- 主题 authority：本文件拥有 mature-world lane、checkpoint、专业化、恢复选择与 anti-grind 的详细玩法语义；不覆盖产品承诺、runtime schema、数值或 Viewer 布局。
- 可变执行状态：对应 GitHub Project task 与 issue evidence comments；当前实现完成度不得由本合同单独宣称。

## 2.9 成熟世界中的小玩家成长线

在 `PostOnboarding` 与首个持续能力之后，产品还必须回答另一件事：当世界已经存在更强组织、更深政治和更长历史时，小玩家/新玩家为什么还值得继续玩。

当前答案不应是“立刻加入大组织”，也不应只是“世界本来就很热闹”。正式路线应当至少提供 1 条不依赖立即站队的 `small-player lane`，让玩家能在成熟世界里继续形成独立 leverage。

当前冻结的长期推荐轴是：

1. `local operator`：先建立并守住 1 条可恢复的小规模工业或服务能力，完成 1 次对世界有可见后果的阶段成果。
2. `regional specialist`：把这条能力转成短周期、区域性有用的专业化服务，而不是马上跳到全局治理或大型宏系统。
3. `limited-scope regional influence`：通过持续贡献获得有限且可审计的局部优先级、机会、可见度或协调位置，但不直接等价为 global governance 权力。

这些轴允许玩家按当前世界状态改道、重排或回退，不是必须逐级完成的职业树。组织、协议或治理等文明尺度项目只能作为自愿的共同扩展：它们可以形成更大范围的协作后果，但不能取代独立成长、成为唯一有效路线或构成全体玩家的胜利条件。

### 2.9.1 Disruption recovery comparison

当 disruption 阻断 active goal 时，gameplay 必须以同一个目标比较 `repair / rebuild / pivot`，并给出每条路线的时间/阶段成本、资源成本、保留/失去价值、主要风险和推荐理由。推荐应解释它如何最快或最可靠地恢复当前目标的玩家价值，而不是只按最低即时成本排序。

- `repair` 保持现有能力并修补关键缺口；`rebuild` 放弃或暂停旧位置/能力后重建同类能力；`pivot` 将已有投入转换为另一项能服务当前目标的区域用途。三者必须有可感知的恢复节奏或价值保留差异。
- 每项比较必须说明独立 `small-player lane` 是否仍可行。仅当独立路径当前确实不可行，才可推荐外部赞助或 major-power 依赖，并说明阻断约束、该依赖的用途及下次重评时机。
- 代表性验收至少覆盖一次局部停机、资源短缺、据点受压或路线失效：玩家能在同一 active goal 下作出有依据的选择，恢复后仍能回到本地立足、区域专业化或有限区域影响，而不是被静默降级为旁观者或强制站队。
- 本节只定义比较与可玩性验收；不新增 runtime 状态、数值、路径可用性算法或 Viewer 布局。

从 `local operator` 切到 `regional specialist` 之前，系统必须展示 `specialization_entry_quote` / `first_delivery_preview`：玩家要知道候选专业化的第一单交付会满足哪个本地需求、预计产出什么、需要哪些输入、多久形成价值、解锁哪种 `leverage_class`，以及交付后的回访 hook。否则专业化只是抽象标签，不能证明 mature-world 小玩家仍有可判断的经营取舍。

- 首单之后的持续区域交付也必须是可选择、可恢复的循环：玩家比较 `fulfill_next_order`、`reserve_capacity_for_local_need`、`reroute_or_pause_service` 与 `exit_specialization`，并看到当前需求、输入/容量窗口、预期区域价值、承诺期限/退出代价和下一次回访动作。缺料、路线、权限、需求失效或接收方不可用时，不得自动续约、吞没未交付库存或把推荐当成交付；玩家可 repair/rebuild/pivot、持有或重新报价。preview 只读且不授予长期市场资格/排他权；正向只产生一次可追溯交付与需求变化，负向/重连/replay 保持未交付状态。该补充不冻结服务费、质量、容量、合约公式或当前 runtime/Viewer 实现声明。
每个 small-player lane checkpoint 还必须展示 `leverage_checkpoint_summary`：`checkpoint_id`、`previous_leverage_class`、`new_leverage_class`、`new_option_unlocked`、`regional_usefulness_delta`、`recovery_resilience_delta`、`negotiation_position_delta`、`same_loop_repeat_count`、`grind_risk_reason`、`recommended_next_branch`、`leverage_checkpoint_class`。该 summary 需要把结果分类为 `new_option_unlocked / resilience_improved / negotiation_position_improved / regional_usefulness_increased / grind_only`；如果只展示 throughput、库存或同一产线重复执行，而没有新选择、恢复弹性、议价位或区域用途，不能判定为 small-player lane progression。

- 中期新增产能必须先给出 `expand_capacity_preview`：玩家比较 `add_parallel_line`、`upgrade_existing_line`、`stabilize_before_expand` 与 `defer`，并看到当前需求、输入/电力/物流可达性、预期新选择或区域用途、承诺/暴露的机会成本和回退动作。成功收益是打开不同的产能或服务路线；容量、权限、输入或电力不足时不得静默扩张、透支现有产线或伪造吞吐，玩家可修复、降载、等待或改道。preview 只读、不生成免费产出/重复里程碑；正向只形成一次可归因能力变化，负向/重连/replay 保持原产能。详细的已建工厂能力升级、重配置与退役选择见 [工厂能力生命周期合同](./gameplay-industrial-factory-capability-lifecycle-contract.prd.md)，仅在 profile 支持时表达且不改变本行扩容语义；该补充不冻结容量/产率/物流公式、状态机或当前实现声明。
这里所谓 `protected first industrial win`，保护的不是“不会被碰”，而是：

- 早期 footprint 小，不应一开始就与 major-power 主战略面重叠。
- 失败后存在 repair / rebuild / pivot 路径，不会立刻把玩家打回“只能投靠别人或只能退坑”。
- 玩家必须能明确回答“我做了什么、世界因此变了什么、下一步为什么仍值得继续”，而不是只看到世界自己在运转。
- 这条线不能只靠“再多做一点同样的工业”维持；每一阶段都必须新增一个 leverage class，例如更稳的恢复权、更短的交付周期、更有议价能力的局部服务位，或新的区域性选择权。
- 如果继续玩唯一能得到的只是更高产量、库存、吞吐或重复次数，而没有新的局部用途、恢复弹性、协调位置或选择空间，这条线应判定为 grind-only，而不是 mature-world lane 成立。
- 如果专业化推荐缺少第一单交付预览，应标记 `specialization_delivery_preview_missing`，不得只用 `recovery_operator` / `conversion_specialist` / `regional_service_runner` 标签替代玩家侧收益说明。

这条线与当前 `PRD-GAME-012` 的 early-retention 冲刺边界保持分离：

- 当前 trust gate / first capability gate 仍是最近两周主优先级。
- `#165` 解决的是“首个持续能力之后如何继续有独立价值”，不是重新改写首个 10 分钟。
- 只有当成熟世界下的小玩家样本能持续给出 `player leverage != world activity only` 的证据时，这条线才算正式成立。

产品承诺见 [`doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`](../../product/world-rules-core-gameplay/mature-world-progression.prd.md)；本节拥有 lane、checkpoint、专业化与 anti-grind 的玩法合同。

Agent-facing 专业执行合同继续消费 `small_player_lane_id`、`leverage_class`、`same_loop_repeat_count`、`grind_only_flag`、`major_power_dependency_status`、`recovery_path_kind`、`requires_major_power_sponsorship` 以及 repair / rebuild / pivot 可用性，并保留 `selected_specialization_id`、`specialization_reason`、`preferred_next_action_class`、`dependency_boundary`、`recovery_escalation_reason` 等可解释摘要；若 guardrail 改写决策，还必须保留 `decision_rewrite` receipt。执行顺序默认先维持 `local_operator`，再按区域需求进入恢复、转换或区域服务专业化；只有玩家自愿升级，或 runtime 明确标记独立路线不可行时，才允许把 major-power dependency 作为有原因的升级路径。当 `same_loop_repeat_count >= 3` 且 leverage 仍是 `throughput_only` / `unclassified` 时，必须停止强化同一循环。

以下情况属于 blocker：独立路径可用时仍把 sponsor / alliance 写成必需；`grind_only` 后继续强化同一 throughput 循环；第一项专业化直接跳到全局治理、联盟领导或战争。没有 bounded canonical trigger 和复查时机时，`wait / wait_ticks` 不能替代 repair / rebuild / pivot。专业化预览缺少本地需求、第一项产出、交付时机或 leverage 解锁时，继续标记 `specialization_delivery_preview_missing`。
