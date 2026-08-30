# 工业首产物结算合同

- 上层产品映射：本合同承接 `doc/game/gameplay/gameplay-top-level-design.prd.md` §2.5 的首个工业目标，并与 `doc/product/world-rules-core-gameplay/prd.md` 的资源/生产权威边界、`doc/world-simulator/m4/industrial-resource-flow-contract.prd.md` 的生产与终端结算边界对齐。
- 主题 authority：本文件只拥有“首个工业目标如何完成、失败后如何恢复、如何打开下一步”的详细玩法语义；不覆盖上层产品承诺、runtime schema 或终端实现。
- 可变执行状态：对应 GitHub Project task 与 issue evidence comments；当前实现完成度不得由本合同单独宣称。

## 1. 目标与术语

首个制成品必须在排程预览或确认前绑定唯一的玩家语义 `starter_completion_profile`。预览至少说明 evidence grade、完成边界、`next_action`、`next_recheck` 与 `progression_effect`。`RecipeScheduled`、accepted 或库存变化都不是生产完成证据。

本合同支持两个显式 profile：`production_only` 把匹配的生产 receipt 作为首产物完成；`terminal-admission` 要求匹配的 delivery/terminal settlement receipt。未声明或无法由 authority 证明 profile 时，必须 fail closed。

## 2. 玩家闭环

玩家先选择目标 profile，查看配方、原材料、能源、物流/终端前置和当前 blocker，再确认排程。成功收益是获得可追溯的生产结果或终端资格，并看到下一步工业用途、交易预览或交付选择；失败成本必须是可读的等待、容量、权限、材料、能源、路线或机会成本，而不是静默扣除。

玩家下一步只能来自权威阶段：等待匹配 receipt、补料/补电、修复前置、取得容量、交付、持有、改道、重报价或延期。系统不得把推荐、排队、accepted 或 terminal pending 伪装成完成，不得自动销毁、改道、退款或发放未结算收益。

## 3. 五阶段结算表

| 阶段 | 玩家动作 | 进度/收益/下一动力 | 失败成本与恢复边界 |
| --- | --- | --- | --- |
| `accepted/scheduled` | 查看生产回执并按 profile 等待；修复前置、补料/补电、改道、重报价或延期 | 仅证明意图接受；不推进首产物、稳定窗口、交付需求、奖励或下一目标 | 状态漂移只能原子拒绝或重报价；不得当作生产完成 |
| `production_settled` | 仅 `production_only` 可完成首产物；随后继续稳定产线或打开交易/交付预览 | 一个 matching production receipt 至多完成一次，结果标为 `production-stable/undelivered`，动力转为首笔交易/工业用途 | 无匹配 receipt 保持待决；不减少交付需求，不发 terminal 奖励 |
| `terminal_pending` | 等待/取得终端容量、交付、持有、改道、重报价或延期（profile 支持时） | 生产结果存在但终端未结算；不减需求、不发交付奖励，玩家承担库存/容量占用和延迟 | owner、资格、容量或路线失效时保留有界 pending/hold；不得免费销毁、自动改道或退款 |
| `delivery/terminal_settled` | 仅显式 `terminal-admission` 可进入交易、区域服务或下一工业目标 | matching delivery/terminal receipt 至多完成一次 profile，并打开一次下一 beat | 不同 root、非 matching、旧 receipt 或 replay 不得重复目标、奖励或释放容量 |
| `profile/authority unknown` | 建立/选择有效目标，或等待补证后复查 | 返回 `no_safe_starter_chain`；不产生排程、sink、稳定进度、奖励或下一 beat | 不得默认 profile，不得用 `0`、空缺口或“已完成”填充缺失 authority |

## 4. Current/target evidence cutline

当前窄实现的 bounded evidence 只有：`crates/oasis7/src/simulator/kernel/actions_resolution.rs:558-632` 的 `ScheduleRecipe` 资源扣除与 `RecipeScheduled`；`crates/oasis7/src/simulator/kernel/replay.rs:379-423` 仅回放电力/数据变化；`crates/oasis7/src/simulator/llm_agent/behavior_loop.rs:1071-1073` 可能在 `RecipeScheduled` 即标记 recipe coverage。它们只能支撑 accepted/resource result，不能支撑 production、delivery、需求减少、奖励或稳定窗口。

目标 evidence 必须由 fresh composite runtime + QA proof 证明：matching production receipt 能唯一对应 root/profile 并完成 `production_only`，terminal profile 能区分 pending 与 settled，且 receipt、需求、奖励和下一目标具有可追溯 authority。取得该证据前，Starter Industrial Feasibility Gate 一律为 `no_safe_starter_chain`；本合同不把 target 当成 current claim。

## 5. Exactly-once、replay 与跨 surface 验收

- 同一 root、profile 与 matching receipt 只能推进一次；重复确认、重连、Agent retry、snapshot restore、乱序事件与 replay 必须返回原处置，不得复制 production、delivery、需求减少、奖励、`W` 或下一目标解锁。
- Viewer、pure API 与 Agent 必须对 profile、阶段、primary blocker、已占用/已消费价值、`next_action`、`next_recheck` 与 `progression_effect` 给出同义结果；任一 surface 缺少 authority 时都显示 `no_safe_starter_chain`，不得自造默认完成。
- 验收必须覆盖：accepted 不推进；`production_only` 的单个 matching production receipt 只完成一次；terminal pending 不发交付收益；terminal settled 只由显式 terminal profile 完成；缺 profile/authority fail closed；报价/权限/容量漂移、重复提交、重连、乱序与 replay 不复制 sink、receipt、奖励或下一目标。

## 6. 非目标与 residual risk

本合同不新增 runtime/API/schema 字段，不规定物流或终端算法，不拍价格、税费、运输、产消或奖励数值，不设计 UI 布局，也不宣称当前 Viewer、pure API、Agent、runtime 或 QA 已完成。`production_only` 降低首局冷启动歧义，但会把物流/终端动机推迟到下一 beat；后续实现仍需验证 receipt authority、恢复动作和三端 parity。
