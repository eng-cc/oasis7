# LLM 工业采矿闭环与调试补给工具（设计文档）

- 对应设计文档: `doc/world-simulator/llm/llm-industrial-mining-debug-tools.design.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-industrial-mining-debug-tools.project.md`

审计轮次: 5

## 1. Executive Summary
- 将 simulator 的工业链路从“可直接精炼硬件”升级为“先采矿再精炼再建厂/排产”的正确机制，避免开局直接进入建厂生产。
- 在 `llm_bootstrap` 中形成更真实的前置阶段：需要采矿与跨地点移动后，才能稳定进入工厂生产。
- 增加仅供 LLM 调试使用的补给工具：可向背包/位置注入任意资源数量，用于调试闭环与回归场景构造。

## 2. User Experience & Functionality

### In Scope
- simulator 资源类型扩展：新增矿物中间资源（`compound`）。
- simulator 动作扩展：新增采矿动作（`mine_compound`）与调试补给动作（`debug_grant_resource`）。
- `refine_compound` 语义升级：必须消耗 `compound` 与电力，产出硬件。
- `refine_compound` 作为恢复/建厂前置动作时，玩家侧需要提交前 `refine_quote`，说明电力机会成本、硬件净增和第一工业目标关联。
- kernel/replay/event/LLM parser/prompt/tool schema 全链路接线。
- 仅在 LLM debug 模式暴露补给 tool（默认关闭）。
- 补齐 `test_tier_required` 单测与闭环回归。

### Out of Scope
- 不切换到 runtime M4 模块全链路（本次仍在 simulator 语义层闭环）。
- 不重构 viewer UI 渲染。
- 不引入外部调度服务与训练策略。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 资源与动作扩展
- `ResourceKind` 新增：
  - `compound`（单位：克，整数）。
- `Action` 新增：
  - `MineCompound { owner, location_id, compound_mass_g }`
  - `DebugGrantResource { owner, kind, amount }`

### 2) 采矿语义（生产机制）
- 采矿前置条件：
  - `compound_mass_g > 0`。
  - `owner` 与目标 `location_id` 共址。
  - 目标 location 具备 `fragment_budget`。
- 采矿成本与约束（配置化）：
  - `mine_electricity_cost_per_kg`：采矿电力成本。
  - `mine_compound_max_per_action_g`：单次采矿上限。
  - `mine_compound_max_per_location_g`：单 location 累计可采上限（用于迫使迁移采集）。
- 采矿结算：
  - 从 location `fragment_budget` 与 chunk budget 扣减元素质量。
  - 向 owner 增加 `ResourceKind::Compound`。
  - 记录 location 累计已采质量。

### 3) 精炼语义升级
- `RefineCompound` 在原电力成本基础上新增硬约束：
  - owner `compound` 库存必须 `>= compound_mass_g`。
  - 扣减 `compound_mass_g` 后再产出 `hardware`。
- 保持已有电力成本和硬件产率配置（向后兼容）。
- 玩家侧 `refine_quote` / `refine_preview` 合同：
  - `compound_mass_g`
  - `electricity_cost`
  - `hardware_output`
  - `electricity_after`
  - `hardware_shortfall_before`
  - `hardware_shortfall_after`
  - `first_goal_relevance`
  - `recommended_refine_amount`
  - `refine_value_class`: `enough_for_next_step / partial_progress / poor_power_tradeoff`
- Edge case: 当 `refine_compound` 被推荐为硬件不足恢复动作，但缺少电力成本、硬件产出或目标缺口变化说明时，标记为 `refine_quote_missing`。
- Acceptance: 玩家在精炼前能看懂“花这些电，换来的 hardware 是否足够推进当前首个工厂/制成品目标”；若精炼后仍不足，需说明继续采矿、先补电或减少精炼量。

### 4) 事件与回放
- `WorldEventKind` 新增：
  - `CompoundMined { owner, location_id, compound_mass_g, electricity_cost, extracted_elements }`
  - `DebugResourceGranted { owner, kind, amount }`
- `kernel/replay` 必须支持新增事件的确定性回放，保证快照/日志一致。

### 5) LLM Debug 工具（仅 debug 模式）
- 新增配置：
  - `OASIS7_LLM_DEBUG_MODE`（默认 `false`）。
- tool 暴露策略：
  - 默认仅保留现有 query tools + `agent_submit_decision`。
  - 当 `debug_mode=true` 时，追加 `agent_debug_grant_resource` tool。
- tool 参数：
  - `owner`（`self|agent:<id>|location:<id>`）
  - `kind`（资源类型）
  - `amount`（`>=1`）
- 安全约束：
  - 非 debug 模式下，即使模型构造该决策也应拒绝执行（解析/守卫层报错并回退）。

## 5. Risks & Roadmap
- MMD0：设计文档与项目管理文档。
- MMD1：采矿与精炼机制落地（含 kernel/replay/tests）。
- MMD2：debug 补给 tool 仅在 debug 模式暴露（含 parser/schema/tests）。
- MMD3：`llm_bootstrap` 闭环复跑、文档/devlog 收口。

### Technical Risks
- 兼容风险：新增 `ResourceKind` 与事件可能影响旧断言与日志消费方。
- 行为风险：location 采矿上限过低会导致策略卡死；过高会退化为“原地采矿”。
- 可读性风险：`refine_compound` 若只作为硬件不足的自动恢复动作出现，玩家可能把扣电理解为隐藏转换税。
- 调试风险：debug tool 暴露边界不严会污染线上策略评估口径。
- 回归风险：LLM guardrail 与新动作并存可能引入未覆盖的决策重写路径。

## MMD5 增量优化（TODO-10 ~ TODO-13）

### 目标
- 提升 `llm_bootstrap` 闭环稳定性，减少 `schedule_recipe`/`move_agent` 高频拒绝，确保“所有工厂 + 所有制成品”目标可持续推进。
- 将“恢复链路”从单步参数猜测改为与经济约束一致的可执行策略，避免无效抖动。
- 在不依赖外部调度器前提下，通过 guardrail 和记忆实现 recipe 覆盖硬切换。

### 范围

#### In Scope
- TODO-10：新增 recipe 覆盖进度记忆（已完成/待完成），并在 `schedule_recipe` 守卫中对已覆盖配方执行硬切换。
- TODO-11：`schedule_recipe` 增加“工厂地点可达性”前置检查，已知工厂地点不共址时优先改写为移动动作。
- TODO-12：`move_agent` 增加分段规划守卫，单步距离超限时自动降级为中继点移动。
- TODO-13：`schedule_recipe -> refine_compound` 恢复质量与 `mine_compound` 单次上限对齐，必要时优先回退到 `mine_compound`。
- 补齐 `test_tier_required` 下的 LLM agent 单测与 prompt 断言。

#### Out of Scope
- 不改 kernel 原子动作语义（不调整真实经济参数与拒绝条件）。
- 不引入全局地图寻路模块（仅在 LLM guardrail 层做局部分段）。
- 不改 viewer 交互面板与可视化样式。

### 接口 / 数据
- `LlmAgentBehavior` 新增轻量运行态：
  - recipe 覆盖状态（completed/missing）。
  - 已知工厂位置映射（factory_id -> location_id）。
- `schedule_recipe` guardrail 扩展：
  - owner=self 且工厂已知不共址时，改写为 `move_agent`（必要时分段）。
  - 硬件不足恢复时按 `mine_compound_max_per_action_g` 限制恢复质量。
- `move_agent` guardrail 扩展：
  - 目标地点可见且距离超过 `max_move_distance_cm_per_tick` 时，改写到可达中继 location。
- prompt 观察上下文新增 recipe 覆盖摘要，用于提示模型主动切换未覆盖配方。

### 里程碑
- MMD5.1：文档更新与任务拆解。
- MMD5.2：TODO-11/12/13 守卫与测试落地。
- MMD5.3：TODO-10 覆盖记忆 + prompt 约束落地。
- MMD5.4：`llm_bootstrap` 在线抽样回归与文档/devlog 收口。

### 风险
- 守卫强改写风险：过度改写可能压制模型探索，导致局部最优。
- 位置映射风险：工厂位置缓存若不及时更新，可能导致错误回退。
- 分段移动风险：可见地点稀疏时中继选择失败，仍可能出现距离超限拒绝。
- 覆盖策略风险：硬切换可能降低单一配方产量，应限定为“未覆盖优先”而非“永久禁止重复”。

## MMD6 增量优化（TODO-14 ~ TODO-16）

### 目标
- 降低 `facility_not_found` 与 `facility_already_exists` 抖动，避免因 `factory_id` 歧义导致的重复建厂/排产失败。
- 在目标 location 不可见或上一轮移动已超距时，提供可执行分段回退，进一步压降 `move_distance_exceeded`。
- 为采矿动作补充“矿点可采量记忆”，在矿点耗尽后主动迁移或降级，降低重复 `mine_compound` 空耗。

### 范围

#### In Scope
- TODO-14：`factory_id` 归一化与去重 guardrail。
  - `schedule_recipe` 支持把“factory_kind 误填为 factory_id”映射到已知工厂 ID。
  - `build_factory` 在已知同 ID 工厂存在时改写为生产推进动作，避免重复建造拒绝。
- TODO-15：`move_agent` 超距回退强化。
  - 记录 `move_distance_exceeded` 目标；
  - 对“已知超距目标 + 不可见目标”应用探索式中继移动。
- TODO-16：采矿耗尽感知。
  - 记录 location 侧 `compound` 可用量（来自拒绝回执）；
  - `mine_compound` 质量按已知可用量裁剪；
  - 当目标点已知耗尽时改写为迁移或电力恢复动作。
- 补齐 `test_tier_required` 单测，并进行 `llm_bootstrap` 在线复核。

#### Out of Scope
- 不修改 kernel 真实拒绝规则与地图生成逻辑。
- 不引入全局寻路器或多步规划器。
- 不调整 recipe 配方数值平衡。

### 接口 / 数据
- `LlmAgentBehavior` 新增运行态记忆：
  - 工厂类型到已知工厂 ID 的映射（用于 `factory_id` 归一化）。
  - 超距移动目标集合（用于后续回退）。
  - 位置可采 `compound` 可用量提示（来自 `InsufficientResource` 回执）。
- guardrail 扩展：
  - `build_factory` 去重改写；
  - `schedule_recipe.factory_id` 归一化；
  - `move_agent` 对已超距目标启用探索中继；
  - `mine_compound` 位置前置检查、质量裁剪与耗尽回退。

### 里程碑
- MMD6.1：文档增量设计与任务拆解。
- MMD6.2：`factory_id` 归一化与建厂去重 guardrail + 单测。
- MMD6.3：移动超距回退增强 + 单测。
- MMD6.4：采矿耗尽感知 guardrail + 单测。
- MMD6.5：`test_tier_required` + 在线闭环抽样复核与文档收口。

### 风险
- 归一化风险：错误映射 factory_id 可能把动作导向非目标工厂，需要优先使用“明确已知映射”。
- 回退风险：探索式中继可能出现局部往返，需要保持“已超距目标”触发条件而非全局强制。
- 采矿记忆风险：可采量提示是时点信息，若 chunk 补给后未清理缓存，可能误判耗尽。

## MMD8 增量优化（TODO-17 ~ TODO-20）

### 目标
- 将“覆盖后持续产出”从 prompt 临时约束下沉为默认 guardrail 行为，降低对用户自定义 prompt 的依赖。
- 收敛 tool-only 协议下的多段/多调用异常，避免 `parse_error` 直接转空转。
- 在 `schedule_recipe` 与 `move_agent` 前置电力预算检查，减少 `insufficient_resource.electricity` 执行期拒绝。
- 为采矿耗尽点增加短期冷却窗口，避免在同一耗尽点反复触发失败后再恢复的抖动路径。

### 范围

#### In Scope
- TODO-17：`wait/wait_ticks` 在 recipe 全覆盖后自动改写为“持续产出”动作（优先 `schedule_recipe`，不可执行时自动回切恢复链路）。
- TODO-18：单轮多 tool call 硬拒绝 + `schedule_recipe.batches<=0` 输入归一化，减少协议噪声与非法参数 parse 错误。
- TODO-19：`schedule_recipe`/`move_agent` 电力前置预算 guardrail（预算不足时回切 `harvest_radiation`）。
- TODO-20：采矿耗尽点冷却窗口（location 级短期跳过）与替代矿点优先策略。
- 补齐 `test_tier_required` 单测与 `llm_bootstrap` 在线抽样。

#### Out of Scope
- 不改 kernel 经济参数与拒绝语义（仅在 LLM guardrail 层优化）。
- 不新增地图级全局寻路器。
- 不改 OpenAI SDK 或第三方依赖实现。

### 接口 / 数据
- `LlmAgentBehavior` 运行态扩展：
  - 采矿耗尽冷却表（`location_id -> cooldown_until_time`）。
- guardrail 扩展：
  - `apply_decision_guardrails` 增加“覆盖后 wait 改写持续产出”分支。
  - `apply_action_guardrails(schedule_recipe)` 增加电力预算约束与 `batches` 下界保护。
  - `apply_action_guardrails(move_agent)` 增加移动电力预算预检。
  - 采矿 guardrail 读取冷却表，冷却期内优先替代矿点或回切恢复动作。
- 协议解析扩展：
  - 单轮多个 completion turn 时输出硬拒绝错误；
  - `schedule_recipe.batches<=0` 解析归一到合法下界，避免直接 parse 失败。

### 里程碑
- MMD8.1：文档增量设计与任务拆解。
- MMD8.2：协议收敛（TODO-18）+ 单测。
- MMD8.3：电力前置预算 guardrail（TODO-19）+ 单测。
- MMD8.4：持续产出默认化 + 采矿冷却（TODO-17/TODO-20）+ 单测。
- MMD8.5：required-tier + 在线闭环抽样复核与文档收口。

### 风险
- 强改写风险：覆盖后 wait 自动改写可能压制模型策略探索，需要保留可回退恢复链路。
- 预检偏差风险：guardrail 预算与 kernel 实际开销若存在偏差，仍可能出现少量执行拒绝。
- 冷却窗口风险：冷却窗口过长可能降低矿点重试效率，需要设置温和值并允许新事件清理。

## MMD9 增量优化（TODO-22 ~ TODO-25）

### 目标
- 修复 `execute_until` 在动作重写和动作匹配上的停止条件缺口，避免“失败后仍继续自动执行”。
- 在 kernel 中收紧 `schedule_recipe` 的工厂-配方兼容关系，防止“power 工厂排 assembler 配方”的语义穿透。
- 压降矿点耗尽后的重复采矿失败，降低后半程“采矿/迁移”抖动并提升持续产出稳定性。

### 范围

#### In Scope
- TODO-22：`execute_until` 动作结果匹配覆盖 `mine_compound/refine_compound`（及被 guardrail 改写后的常见动作），确保 `action_rejected`/`last_action_failed` 能及时终止自动执行。
- TODO-23：`schedule_recipe` 增加工厂类型兼容校验（assembler 配方仅允许在 assembler factory 上执行）。
- TODO-24：采矿候选选择增加“失败记忆优先级”，同等条件下避开最近连续失败矿点，减少无效重试。
- TODO-25：当 `execute_until.action` 被 guardrail 改写为不同动作类型时，同步重建默认 `until` 条件，避免沿用旧条件导致重复动作抖动。
- 补齐 `test_tier_required` 单测与 `llm_bootstrap` 在线抽样复核。

#### Out of Scope
- 不改配方数值平衡与经济参数。
- 不引入全局路径规划器。
- 不改 viewer 展示逻辑。

### 接口 / 数据
- `execution_controls`：
  - 扩展 `actions_same` 的动作匹配范围；
  - 支持根据动作类型推导 `until` 默认条件（供 guardrail 改写后重建）。
- `kernel/actions`：
  - 在 `ScheduleRecipe` 执行路径增加 `recipe_id <-> factory.kind` 兼容检查。
- `LlmAgentBehavior`：
  - 增加矿点失败记忆（location 级），并在候选排序中生效。

### 里程碑
- MMD9.1：修复 `execute_until` 动作匹配缺口（TODO-22）+ 单测。
- MMD9.2：落地 `schedule_recipe` 工厂兼容校验（TODO-23）+ kernel 单测。
- MMD9.3：矿点失败记忆与候选排序（TODO-24）+ 单测。
- MMD9.4：guardrail 改写后 `until` 重建（TODO-25）+ 单测与在线抽样。

### 风险
- 兼容风险：动作匹配放宽后，可能影响极少数依赖“动作参数差异”继续执行的旧样例。
- 收敛风险：矿点失败记忆过强可能抑制必要重试，需要设置温和值并在补给事件时清理。
- 行为风险：`until` 重建过于激进可能提前退出自动执行，需通过回归用例覆盖 move/harvest/mine 三类路径。

## MMD11 增量优化（TODO-26 ~ TODO-29）

### 目标
- 解决 `llm_bootstrap` 长窗口中“工厂选择漂移”导致的主失败源：
  - `facility_not_found`（factory_id 别名/伪 ID 未归一）
  - `rule_denied`（配方投递到不兼容工厂）
  - `location_not_found`（build_factory 使用不可识别 location）
- 在不改 kernel 拒绝语义前提下，把修复收敛在 LLM guardrail 层，优先提升动作可执行率与闭环稳定性。

### 范围

#### In Scope
- TODO-26：`schedule_recipe` 的 factory_id 归一增强。
  - 支持 `factory.<kind>.<suffix>` 伪 ID 映射到已知 canonical factory_id。
  - 当 `factory_id` 不可识别时，按 `recipe_id` 所需 `factory_kind` 回退到已知工厂。
- TODO-27：`schedule_recipe` 前置 `recipe -> factory_kind` 强校验。
  - 发现不兼容时优先改写到兼容工厂；
  - 若兼容工厂缺失，改写为 `build_factory(required_kind)`，不再把无效排产直接下发 kernel。
- TODO-28：`build_factory.location_id` 合法化与同位预检。
  - 不可见/不可识别 location 自动回退到当前 location；
  - 可见但不同位时优先改写为 `move_agent`。
- TODO-29：补充失败语义透传（`rule_denied`、`location_not_found`）与 prompt 恢复策略提示，降低“reject_reason=other”导致的恢复误判。
- 补齐 `test_tier_required` 单测与在线 `llm_bootstrap` 对照复验。

#### Out of Scope
- 不修改 kernel 中 `schedule_recipe` 工厂兼容拒绝规则。
- 不调整经济参数（配方成本、电力成本、采矿参数）。
- 不改 viewer 展示层与场景生成逻辑。

### 接口 / 数据
- `LlmAgentBehavior` guardrail 扩展：
  - `schedule_recipe`：增加 required factory kind 推导、factory_id 归一增强、缺厂时建厂回退。
  - `build_factory`：增加 location 归一与同位移动预检；去重改写增加工厂类型兼容判断。
- prompt 反馈扩展：
  - `RejectReason::RuleDenied` -> `rule_denied`
  - `RejectReason::LocationNotFound` -> `location_not_found`
  - 更新 `[Failure Recovery Policy]`：新增 `rule_denied` / `location_not_found` 的恢复建议。

### 里程碑
- MMD11.1：文档增量设计与项目任务拆解。
- MMD11.2：guardrail 修复（TODO-26/27/28）+ required-tier 单测。
- MMD11.3：prompt/reject_reason 透传收敛（TODO-29）+ 单测。
- MMD11.4：在线 `llm_bootstrap --ticks 120` 对照复验 + 文档/devlog 收口。

### 风险
- 改写激进风险：过度自动改写可能压制模型探索，需保留 note 与 trace 可审计。
- 错误归一风险：factory/location 归一若命中错误对象，可能引入新的策略偏差；需以“已知集合”优先并在不可判定时回退最保守动作。
- 行为漂移风险：prompt 恢复策略新增分支可能影响既有样本，需要 required-tier + 在线样本双口径回归。

## MMD12 P0 再验证与闭环修复（TODO-28 / TODO-29）

### 目标
- 在 MMD11 后按同口径 120 tick 复验，确认两条 P0 是否仍存在：
  - P0-A：多段输出协议不稳导致 `parse_errors` 与 `decision_wait` 升高。
  - P0-B：`build_factory` 使用伪 location 别名导致 `location_not_found`。
- 若仍存在，完成最小改动修复并复验到可量化收敛（优先关注 `parse_errors`、`location_not_found`、`decision_wait`）。

### 范围

#### In Scope
- `LlmAgentBehavior` 对多段输出的 guardrail 收敛：
  - 在单轮解析出多段 payload 时，优先保留“最后一个终态 turn（decision/execute_until/decision_draft）”。
- `build_factory` location guardrail 收敛：
  - 当请求 location 不可见时，优先回退到“严格当前位置（distance=0）”，若缺失则回退到“最近可见 location”。
  - 当不可见且无法推断任何当前位置时，回退到保守动作 `harvest_radiation`，避免直接下发高概率拒绝动作。
- 补齐/更新 `test_tier_required` 相关单测，并执行同口径在线复验。

#### Out of Scope
- 不修改 kernel 拒绝语义与经济参数。
- 不引入新的全局规划策略，仅修补 guardrail 与协议处理。
- 不改 viewer 协议或展示逻辑。

### 接口 / 数据
- `crates/oasis7/src/simulator/llm_agent/behavior_loop.rs`
  - `collapse_multi_turn_payloads` 与 `parsed_turn_kind_name` 作为 `LlmAgentBehavior` 固有 helper，服务单轮多段 payload 折叠。
- `crates/oasis7/src/simulator/llm_agent/behavior_guardrails.rs`
  - `build_factory` guardrail 增加 `inferred_current_location_id`（当前位置优先、最近可见兜底）与不可推断时的保守回退。
- `crates/oasis7/src/simulator/llm_agent/tests.rs` / `tests_part2.rs`
  - 更新旧协议断言为“折叠终态”语义；
  - 新增“当前位置缺失时按最近可见位置归一”的覆盖用例。

### 里程碑
- MMD12.1：同口径在线基线复验（确认 P0 仍存在）。
- MMD12.2：修复多段协议处理 + 编译/单测通过。
- MMD12.3：在线复验 #1（验证协议收敛效果，定位剩余 location 问题）。
- MMD12.4：修复 build location 归一兜底 + 在线复验 #2，完成文档/devlog 收口。

### 风险
- 最近可见位置兜底可能在极端观测误差下误判目标位置，需要依赖后续动作反馈纠偏。
- 多段折叠策略偏“最后终态优先”，可能丢弃前序 module_call 结果；需通过 trace note 保持可审计。
- 在线样本受模型随机性影响，单次 120 tick 仍可能波动，后续可按需要补多样本验证。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
