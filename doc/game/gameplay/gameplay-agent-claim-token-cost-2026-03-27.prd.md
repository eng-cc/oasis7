# Gameplay Agent 认领代币成本与维护机制（2026-03-27）

- 对应设计文档: `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.project.md`

审计轮次: 8

## 1. Executive Summary
- Problem Statement: 当前规则把 agent 认领完全绑定到 `liquid main token`。在“首个 claim 也不免费”生效后，limited preview / allowlist / QA seed 账号若没有可流通余额就无法进入中循环；但直接空投可转账 main token 又会打开刷号和套现路径。
- Proposed Solution: 保持“首个 claim 仍为非零 canonical 成本”的主规则不变，在 main token 账本中新增 `restricted starter claim balance`。该余额不可转账、不可提现、不可用于普通资产动作，只可用于 `slot-1` 的 claim / upkeep；由其资助的 bond refund 必须退回同一 restricted bucket，不得洗成可流通余额。
- Success Criteria:
  - SC-1: 第 1 个 agent 认领仍不存在免费路径；所有成功认领都必须满足 `activation_fee_amount > 0`、`claim_bond_amount > 0`、`upkeep_per_epoch > 0`。
  - SC-2: `restricted starter claim balance` 仅允许用于 `slot-1` 的 upfront claim cost 与 `slot-1` upkeep；其通过 `TransferMainToken`、公开转账 API 或 explorer 导出的误放过率为 `0`。
  - SC-3: 任一 agent 在同一时刻只能有 1 个正式 `claim_owner_id`；并发争抢、重复认领和无成本续占的误放过率为 `0`。
  - SC-4: 单账号可同时认领的 agent 数在 v1 受 `reputation_tier` 限制为 `1/2/3` 三档，且 `slot-2/slot-3` 的总成本分别至少为 `slot-1` 的 `1.5x/2.0x`。
  - SC-5: 使用 restricted bucket 资助的 bond，在 release / reclaim / slash 后必须按 canonical provenance 退回 restricted bucket，不得洗成 `liquid main token`。
  - SC-6: `activation fee`、`upkeep`、`bond refund/slash` 与 restricted grant 的发放/消费/退款/过期必须全部形成可审计事件，并能进入现有 main token 源汇审计链路。
  - SC-7: `IssueRestrictedStarterClaimGrant / RevokeRestrictedStarterClaimGrant` 在进入 runtime grant 状态机前必须先通过正式 `admin registry` 门禁；当 registry 缺失、issuer 未登记或未绑定 signer allowlist policy 时，误放过率为 `0`。
  - SC-8: restricted grant admin registry 必须支持通过正式 runtime action 热更新；仅允许当前 `ecosystem_pool` treasury controller slot 绑定的 controller account 变更 `restricted_starter_claim_admin_account_ids` 子集，且变更 action 必须携带通过 signer allowlist / threshold policy 校验的主链签名 proof。
  - SC-9: `liveops_community` 的日常 restricted grant 发放/撤销/状态检查必须支持正式 CLI 入口，直接复用 runtime canonical action 与 world-state 真值，而不是要求运营手工拼接原始 action JSON 或直接编辑 world 文件。
  - SC-10: 在正式 CLI 之上，仓库还必须提供一层面向运营同事的短命令 wrapper，支持 `status / issue / revoke` 的位置参数和 `OASIS7_WORLD_DIR` 默认注入，同时不新增任何绕过 runtime / controller-governed admin registry 的旁路。
  - SC-11: governance registry 的 manifest/import/audit 工具链必须支持按 `slot_id` 声明独立 signer threshold；`liveops` 这类低权限 restricted grant admin slot 可以显式使用 `1-of-2`，而 treasury/controller 主槽位继续默认 `2-of-3`，且不得因为引入 `liveops` 特例而把其他 slot 一并降阈值。
  - SC-12: daily restricted grant 的出账源必须从 `ecosystem_pool` 切换到独立 `restricted_starter_claim_liveops_pool`；该专用池只能由当前 `ecosystem_pool` treasury controller slot 绑定的 controller account 通过正式 top-up action 从大池划拨资金，`liveops_community` 的日常 `issue/revoke/status` 只消费和回读该专用池，不直接动 `ecosystem_pool`。

## 2. User Experience & Functionality
- User Personas:
  - 中循环玩家/组织经营者：需要用明确代价换取 agent 控制权，而不是靠抢占或挂机囤位。
  - `producer_system_designer`: 需要把 agent 认领从“模糊权限”收成可平衡、可审计的经济规则。
  - `runtime_engineer`: 需要一套可确定执行的 claim / upkeep / reclaim 状态机与记账规则。
  - `viewer_engineer`: 需要在 UI / pure API 中向玩家清楚展示认领成本、受限余额、宽限、冷却和回收风险。
  - `qa_engineer`: 需要验证并发争抢、欠费、闲置、多号囤积、transfer guard 和经济审计没有旁路。
  - `liveops_community`: 需要向 limited preview / allowlist / QA seed 账号发放受限启动余额，并在需要时回收、停用或过期。
  - restricted grant admin operator: 需要一条正式 runtime 真值来判断 `issuer_id=liveops` 是否真的具备发放/撤销权限，而不是只依赖 runbook 约定。
  - restricted grant liveops operator: 需要低摩擦的日常操作入口来 issue/revoke/status restricted grant，而不是每次都理解底层 runtime action 或 controller payload 细节。
  - non-technical liveops operator: 需要更短、更稳定的脚本入口与环境变量约定，减少手工复制长命令时的误填与漏填。
- User Scenarios & Frequency:
  - 首次建立组织能力时：每个认真进入中循环的玩家至少 1 次。
  - limited preview / allowlist 发放时：每轮受控外放、QA 种子建号或运营定向补助时发生。
  - 扩展更多 agent 槽位时：随着玩家声誉提升，多次发生。
  - 日常持有期：每个 upkeep 结算 epoch 都会发生。
  - 释放 / 被回收时：主动退场、欠费或闲置时发生。
- User Stories:
  - PRD-GAME-011: As a 中循环玩家, I want every agent claim to keep a non-zero main-token-denominated cost and require ongoing upkeep, so that agent control reflects actual commitment instead of zero-cost squatting.
  - PRD-GAME-011A: As a `producer_system_designer`, I want the first claim to also be non-free, so that the world does not silently create a “starter free slot” that weakens the sink and encourages alt abuse.
  - PRD-GAME-011B: As a `qa_engineer`, I want forced reclaim and refund/slash outcomes to be deterministic, so that we can test abuse resistance instead of relying on manual moderation.
  - PRD-GAME-011C: As a limited preview / allowlist owner, I want to seed first-claim funds without granting transferable assets, so that test users can enter mid-loop without opening an airdrop abuse lane.
  - PRD-GAME-011D: As a `runtime_engineer`, I want claim/refund accounting to preserve funding-source provenance, so that restricted starter funds cannot be converted into transferable main token through release or reclaim.
- Critical User Flows:
  1. Flow-AGC-001: `liveops / onboarding / QA seed 发放 restricted starter claim balance -> 账户获得不可转账、带用途范围的 canonical bucket`
  2. Flow-AGC-002: `玩家选择未认领 agent -> 系统返回 slot quote（activation fee / bond / upkeep / cap / eligible balances）-> 若为 slot-1 则 restricted 余额优先参与报价 -> 玩家确认 -> 扣除 activation fee、锁定 bond -> agent 进入 claimed_active`
  3. Flow-AGC-003: `每个 upkeep epoch 到达 -> 系统按 restricted -> liquid 优先级尝试结算 slot-1 upkeep -> 余额足够则继续持有 -> 不足则进入 upkeep_grace`
  4. Flow-AGC-004: `玩家在 cooldown 后主动 release -> 系统结清欠费 -> 按原 funding source 退还 bond 剩余部分 -> agent 回到 unclaimed`
  5. Flow-AGC-005: `claim 进入 grace 后仍未补足 upkeep 或连续闲置达到阈值 -> 系统执行 forced_reclaim -> 计算 slash / refund -> 按 funding source 回写退款 bucket -> agent 回到 unclaimed`
  6. Flow-AGC-006: `玩家尝试认领第 2/3 个 agent 或发起普通转账 -> 系统拒绝消费 restricted bucket，并返回结构化 blocker`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Restricted Starter Balance | `account_id`、`restricted_starter_claim_balance`、`issuance_reason`、`spend_scope`、`expires_at_epoch`、`issuer_id` | liveops / onboarding / QA seed 发放或回收受限余额 | `unissued -> issued -> partially_spent -> exhausted/expired/revoked` | `spend_scope` 固定为 `slot-1 claim + slot-1 upkeep`；默认不可与 `TransferMainToken` 共享 | 仅指定 issuer 可发放/回收；持有人不可转赠 |
| Claim Quote | `agent_id`、`claimer_id`、`claim_slot_index`、`reputation_tier`、`claim_cap`、`activation_fee_amount`、`claim_bond_amount`、`upkeep_per_epoch`、`transferable_liquid_balance`、`restricted_starter_claim_balance`、`eligible_claim_balance`、`release_cooldown_epochs` | 玩家查看未认领 agent 时返回成本报价与风险提示 | `unclaimed -> quote_ready` | `slot-1 multiplier=1.0`、`slot-2=1.5`、`slot-3=2.0`；`total_upfront_cost = activation_fee_amount + claim_bond_amount + upkeep_per_epoch` | 仅当 agent 未被认领、玩家未超 cap，且 `eligible_claim_balance >= total_upfront_cost` 时可确认；`slot-2/3` 的 eligible balance 不得包含 restricted bucket |
| Claim Activation | `claim_owner_id`、`claim_started_epoch`、`next_upkeep_epoch`、`claim_bond_locked_amount`、`claim_bond_locked_restricted_amount`、`claim_bond_locked_liquid_amount`、`activation_fee_burn_amount`、`activation_fee_treasury_amount`、`upfront_restricted_spent_amount`、`upfront_liquid_spent_amount` | 点击确认认领后扣费并锁定 bond | `quote_ready -> claimed_active` | `activation_fee_split_bps = 5000 burn / 5000 treasury`；`slot-1` 优先花 restricted，再补 liquid；首个认领也必须扣费 | 同一 agent 同时只能成功 1 个 claim；并发失败方必须原子回滚 |
| Upkeep Settlement | `upkeep_due_epoch`、`upkeep_per_epoch`、`upkeep_paid_amount`、`upkeep_restricted_spent_amount`、`upkeep_liquid_spent_amount`、`grace_deadline_epoch`、`delinquent_amount` | 到达结算 epoch 时自动尝试扣除 upkeep | `claimed_active -> claimed_active` 或 `claimed_active -> upkeep_grace` | `slot-1` upkeep 先尝试 restricted，再尝试 liquid；`slot-2/3` upkeep 只能花 liquid；每次只结算 1 个 epoch 的应付额 | 仅系统结算；owner 可通过补足可用余额恢复 |
| Voluntary Release | `release_requested_epoch`、`release_effective_epoch`、`bond_refund_amount`、`bond_refund_restricted_amount`、`bond_refund_liquid_amount`、`cooldown_satisfied` | owner 主动放弃当前 claim | `claimed_active/upkeep_grace -> released -> unclaimed` | 退款金额 = `claim_bond_locked_amount - unpaid_upkeep - penalties`；restricted 来源的 bond 退款必须回 restricted bucket | 只有当前 owner 可 release；未过 cooldown 不允许 |
| Forced Reclaim | `forced_reason`、`forced_reclaim_epoch`、`forced_penalty_amount`、`bond_refund_amount`、`bond_refund_restricted_amount`、`bond_refund_liquid_amount` | 欠费超宽限或持续闲置时系统回收 | `upkeep_grace/inactive_reclaim_candidate -> forced_reclaimed -> unclaimed` | 欠费宽限 `grace_epochs = 2`；闲置阈值 `7` 个 epoch，最晚 `10` 个 epoch 完成回收；`forced_reclaim_penalty_bps = 2000`（作用于剩余 bond） | 仅系统可执行；owner 不能在最终回收点之后阻断 |
| Transfer Guard | `transferable_balance`、`restricted_balance`、`blocked_reason` | 玩家发起普通 main token 转账或 explorer 导出余额时，系统区分可转账与不可转账余额 | `eligible -> blocked_for_restricted_only` | `transferable_balance = liquid_balance`；`restricted_balance` 只做展示，不计入可转账金额 | `TransferMainToken`、公开转账 API、explorer 排名与总额不得消费或误计 restricted bucket |
| Reputation Cap | `reputation_tier`、`claim_cap`、`owned_agent_count` | 认领前校验当前账号可占有的 agent 上限 | `eligible -> eligible/blocked_by_cap` | `tier-0 cap=1`、`tier-1 cap=2`、`tier-2+ cap=3` | 非法 tier 或超 cap 直接拒绝，不允许只靠余额绕过 |

- Acceptance Criteria:
  - AC-1: 首个 agent 认领没有任何免费分支；v1 必须显式校验 `activation_fee_amount > 0`、`claim_bond_amount > 0`、`upkeep_per_epoch > 0`。
  - AC-2: 认领成功后必须立即形成 `activation fee` 记账、`bond locked` 状态和下一次 upkeep 结算 epoch，不允许“先占坑后补票”。
  - AC-3: 同一 agent 被两个账号并发认领时，只允许一个成功；失败方不得丢失 token、不得产生脏 claim。
  - AC-4: `slot-1` 允许消费 `restricted starter claim balance`；`slot-2/3` claim 与对应 upkeep 不允许消费该 bucket，必须返回结构化 blocker。
  - AC-5: owner 主动 release 时，若已过 cooldown 且无未结债务，必须退回剩余 bond，并按原 funding source 将 restricted/liquid refund 拆分回对应 bucket。
  - AC-6: 欠费 claim 在 `2` 个 epoch 宽限后必须被回收；持续闲置 claim 必须在 `10` 个 epoch 内被回收到未认领池。
  - AC-7: 强制回收必须给出 `forced_reason`、`forced_penalty_amount`、`bond_refund_amount`，并通过统一事件与审计字段可追溯；restricted 来源的 refund 不得回到 liquid。
  - AC-8: Viewer / pure API 必须同时展示 `claim_slot_index`、`activation_fee_amount`、`claim_bond_amount`、`upkeep_per_epoch`、`grace_deadline_epoch`、`release_cooldown_epochs`、`restricted_starter_claim_balance` 与 `eligible_claim_balance`，不允许 UI/API 各算一套。
  - AC-9: `TransferMainToken`、公开转账 API、explorer 余额排序与总额展示必须显式区分 `transferable_balance` 与 `restricted_balance`；restricted 不得被视为可转账资产。
  - AC-10: 本机制不得被表述为现实货币付费解锁、公开售卖 agent 或永久产权出售；它是 gameplay 内部的 main token 承诺成本机制与受限启动资助工具。
- Non-Goals:
  - 不在本专题内定义现实货币购买、法币结算或站外商城。
  - 不把 agent 认领做成永久不可回收的链上产权 NFT。
  - 不在本轮引入代理拍卖行、agent 二级交易市场或跨玩家租赁市场。
  - 不在本轮为 claim 成本拍死绝对 token 数值；v1 先冻结公式、状态机和不可突破的边界。
  - 不把 `restricted starter claim balance` 扩展成通用的“全游戏不可转账 main token”体系。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不新增 AI 模型能力，仅涉及 gameplay 规则、账本 bucket 与状态机）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview:
  - gameplay 层新增 `agent claim economy` 规则：负责报价、claim 状态机、slot multiplier、cap 和回收逻辑。
  - runtime 负责原子扣费、bond 锁定、epoch upkeep 结算、slash / refund 和记账事件，并新增 `restricted starter claim balance` bucket、可消费范围与 refund provenance。
  - viewer / pure API 只读取 canonical claim 状态、报价字段与 restricted/liquid 余额拆分，不自行推导隐藏成本。
  - main token 账本继续作为唯一价值来源；claim 机制允许 `slot-1` 消费 `restricted starter claim balance + liquid main token` 的 canonical 组合，但转账面与公开资产面只消费 `liquid main token`，不旁路 signed action / audit 链路。
- Integration Points:
  - `doc/game/prd.md`
  - `doc/game/project.md`
  - `doc/game/gameplay/gameplay-engineering-architecture.md`
  - `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
  - `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 余额不足：报价可见，但确认认领必须拒绝，并明确缺少的是 `activation fee`、`bond`、`upkeep` 还是 `eligible_claim_balance` 总额不足。
  - restricted grant admin 未登记：当 `governance_main_token_controller_registry.restricted_starter_claim_admin_account_ids` 缺失、为空、或不包含当前 `issuer_account_id` 时，issue / revoke 必须在 grant / treasury / beneficiary 逻辑之前直接拒绝。
  - restricted bucket 过期：已过 `expires_at_epoch` 的 starter balance 不得进入 quote 可用余额，且必须生成可审计的 `expired` 事件。
  - mixed funding：若 upfront cost 同时由 restricted 与 liquid 支付，系统必须记录 bond provenance；后续 refund/slash 结算必须按该 provenance 拆分。
  - 并发争抢：两个提交同时命中同一 `agent_id` 时，只允许第一个写入 `claim_owner_id`；第二个返回冲突，不得重复扣费。
  - upkeep 结算时余额不足：进入 `upkeep_grace`，并写出 `grace_deadline_epoch`；宽限内补足可用余额后恢复 `claimed_active`。
  - `slot-2/3` 误用 restricted：必须拒绝，并返回 `restricted_balance_not_eligible_for_slot` 一类结构化 blocker。
  - release 请求早于 cooldown：拒绝 release，但必须返回剩余 `cooldown_epochs_remaining`。
  - force reclaim 与 owner 同 epoch 操作冲突：以先完成的合法状态迁移为准，后到请求必须基于最新状态重试。
  - 闲置判断：若 `7` 个连续 epoch 无 owner 发起的有效控制动作或 agent 产出的有效推进事件，则进入 `inactive_reclaim_candidate`；若到 `10` 个 epoch 仍未恢复，则强制回收。
  - slash 后 refund 为负：退款下限固定为 `0`，不得从系统额外倒贴。
  - 转账面误读余额：若账户只有 restricted 余额、没有 liquid 余额，普通转账必须拒绝并返回 `transferable_balance=0`，不得因为显示“总余额 > 0”而误放行。
  - tier 异常：无效 `reputation_tier` 一律按最低 tier 处理，不允许读空后默认开放更多槽位。
  - UI/API 语义缺口：若某端未显示 canonical claim 成本与倒计时，或未显示 restricted/liquid 区分，则该端不得宣称支持正式 agent claim 管理。
- Non-Functional Requirements:
  - NFR-AGC-1: 首个 claim 免费路径命中次数必须为 `0`。
  - NFR-AGC-2: 同一 `agent_id` 的并发 claim 误放过率必须为 `0`。
  - NFR-AGC-3: `activation fee`、`upkeep`、`bond refund/slash` 与 restricted grant 发放/消费/退款/过期事件覆盖率必须为 `100%`。
  - NFR-AGC-4: viewer / pure API 在 canonical claim 字段、restricted/liquid 余额字段和 blocked reason 上的一致性必须为 `100%`。
  - NFR-AGC-5: 宽限到强制回收的检测延迟 P95 必须 `<= 1 epoch`。
  - NFR-AGC-6: 单账号 agent cap 默认不得超过 `3`，除非后续新 PRD 明确升级。
  - NFR-AGC-7: v1 的成本曲线必须保持单调不降，不允许出现“第 2 个 agent 比第 1 个更便宜”的参数。
  - NFR-AGC-8: restricted bucket 通过普通转账路径、公开资产导出路径或 slot-2/3 claim 路径的误放过率必须为 `0`。
  - NFR-AGC-9: 所有 claim 相关 token 变动必须能进入现有经济源汇审计，不得生成审计盲区。
  - NFR-AGC-10: restricted grant admin registry 的 source of truth 必须来自 runtime world-state 正式 registry，而不是文档约定、自由文本 `issuer_id` 或调用方本地分支逻辑。
  - NFR-AGC-11: restricted grant admin registry runtime update 必须可审计、可重放，并在 registry 尚未 bootstrap、`ecosystem_pool` controller slot 未配置、或提交者不匹配当前 controller account 时拒绝执行，不得退化为离线 import 才能变更的单一路径。
- Security & Privacy:
  - claim / release / upkeep 结算不得绕过主链 token 的签名与审计路径。
  - 不在公开 UI 中暴露与 claim 无关的账户私密资产信息；只展示本次认领所需的必要成本和状态。
  - 强制回收必须基于可重放的状态与事件，不允许人工后台静默改 owner。
  - restricted starter balance 的 issuer、用途范围与过期处理必须可审计，且不得成为绕过主账本 transfer auth 的隐式旁路。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 冻结 `activation fee + claim bond + upkeep + release cooldown + tier cap` 的规则口径与状态机。
  - v1.1: 落地 `restricted starter claim balance`、canonical viewer / pure API 展示和 QA abuse suite。
  - v2.0: 基于真实 claim 数据评估 `slot multiplier`、`grace_epochs`、`penalty_bps` 与 starter balance 发放上限是否需要新一轮调参专题。
- Technical Risks:
  - 风险-1: 如果 claim 成本只锁 bond 不产生真实 sink，囤位问题可能仍然偏轻处罚。
  - 风险-2: 如果 upkeep 过高，会让 agent 控制变成纯惩罚，削弱组织扩张乐趣。
  - 风险-3: 如果 viewer 不清楚展示倒计时与宽限，玩家会把强制回收理解为 bug 而不是规则。
  - 风险-4: 如果 tier cap 只靠离线判断、不进 runtime，alt 账号仍可能形成事实囤积。
  - 风险-5: 如果 restricted bucket 被设计成通用不可转账 main token，而不是受限于 `slot-1 claim/upkeep` 的窄用途 bucket，会把 claim 启动补贴扩散成第二套资产语义。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-GAME-011 | `TASK-GAME-039` | `test_tier_required` | 文档治理检查、根入口/索引/任务映射核验 | agent claim 经济专题挂载 |
| PRD-GAME-011 | `TASK-GAME-040` | `test_tier_required` + `test_tier_full` | claim / upkeep / release / forced reclaim 状态机回归、经济审计对账、并发争抢测试 | runtime 规则执行、token 记账、安全边界 |
| PRD-GAME-011 | `TASK-GAME-041` | `test_tier_required` | Viewer / pure API canonical 字段、报价展示、宽限/冷却倒计时回归 | 玩家表达层、UI/API 一致性 |
| PRD-GAME-011 | `TASK-GAME-042` | `test_tier_required` + `test_tier_full` | abuse suite、长稳回收、经济告警与不变量复核 | QA 守门、反囤积、经济审计 |
| PRD-GAME-011 | `TASK-GAME-043` | `test_tier_required` | producer 平衡复盘、调参边界与继续/回退决策回写 | 版本平衡、后续节奏裁决 |
| PRD-GAME-011 | `TASK-GAME-044` | `test_tier_required` | PRD / design / project / root doc 回写、术语一致性与追溯链核验 | restricted starter claim balance 规则入档 |
| PRD-GAME-011 | `TASK-GAME-045` | `test_tier_required` + `test_tier_full` | restricted bucket 账本、claim/upkeep/refund provenance、transfer guard、snapshot/replay 兼容回归 | runtime 账本、资金来源约束、安全边界 |
| PRD-GAME-011 | `TASK-GAME-046` | `test_tier_required` | Viewer / pure API 余额拆分、funding mix、blocked reason 与 explorer 展示回归 | 玩家表达层、资产可读性、UI/API 一致性 |
| PRD-GAME-011 | `TASK-GAME-047` | `test_tier_required` + `test_tier_full` | allowlist/QA seed 发放、slot-1/slot-2 guard、refund provenance、过期/回收与经济审计 matrix | QA 守门、反滥用、经济审计 |
| PRD-GAME-011 | `TASK-GAME-048` | `test_tier_required` | producer 对 starter balance 额度、过期与发放边界的首轮复核与继续/回退决策 | limited preview 节奏、版本平衡、运营口径 |
| PRD-GAME-011 | `TASK-GAME-049` | `test_tier_required` + `test_tier_full` | restricted grant 的 `issuance_reason / issuer_id / expires_at_epoch` 状态、issue/expire/revoke 事件、issuer-scoped 发放/回收动作与 token audit linkage 回归 | runtime grant lifecycle、经济源汇审计、资金来源约束 |
| PRD-GAME-011 | `TASK-GAME-050` | `test_tier_required` | liveops issuer 边界、allowlist/QA seed/campaign 发放口径、过期/撤销 runbook 与 incident fallback 核验 | 运营发放、对外口径、风险收敛 |
| PRD-GAME-011 | `TASK-GAME-051` | `test_tier_required` + `test_tier_full` | restricted grant lifecycle / audit matrix、expiry/revoke/source-sink 对账与 non-bypass 验证 | QA 守门、反滥用、经济审计 |
| PRD-GAME-011 | `TASK-GAME-052` | `test_tier_required` + `test_tier_full` | governance main-token controller registry 补齐 restricted grant admin registry / signer allowlist 绑定，并验证 registry 缺失、非 admin、非 allowlisted issuer 与 allowlisted admin 的 runtime action gate | runtime admin 真值、发放/撤销入口门禁、运营安全边界 |
| PRD-GAME-011 | `TASK-GAME-053` | `test_tier_required` + `test_tier_full` | 新增 `UpdateRestrictedStarterClaimAdminRegistry` controller-governed runtime action，验证 `ecosystem_pool` controller account 绑定、signer policy 约束、governance event apply 与“先更新 registry 再发 grant”的闭环 | runtime admin registry 热更新、主链钱包治理审计、运营解锁链路 |
| PRD-GAME-011 | `TASK-GAME-054` | `test_tier_required` | 新增 `oasis7_liveops_grant_cli`，封装 restricted grant 的 `issue/revoke/status` 日常操作，验证 CLI 仍复用 canonical runtime action / world-state 真值，且不开放 admin roster 直改旁路 | 运营操作降摩擦、runtime 真值复用、治理边界保持 |
| PRD-GAME-011 | `TASK-GAME-055` | `test_tier_required` | 新增 `scripts/oasis7-liveops-grant.sh` 作为运营 wrapper，验证位置参数、`OASIS7_WORLD_DIR` 默认注入与 `--print-cmd/--cli-bin` smoke，同时保持底层仍只调用 `oasis7_liveops_grant_cli` | 运营执行门槛进一步降低、字段误填率下降、治理边界不扩散 |
| PRD-GAME-011 | `TASK-GAME-056` | `test_tier_required` | 将 governance registry manifest/import/audit 扩成 per-slot threshold，验证 `liveops` 可显式声明 `1-of-2` 且通过 audit/import，而既有 controller/finality slot 继续保持默认 `2-of-3` 与单 signer 故障容忍审计 | 低权限运营槽位可正式落地、治理工具链不再全局绑死一个 threshold |
| PRD-GAME-011 | `TASK-GAME-059` | `test_tier_required` + `test_tier_full` | 新增 `TopUpRestrictedStarterClaimLiveopsPool` controller-governed runtime action、独立 `restricted_starter_claim_liveops_pool` treasury bucket 与 top-up 审计记录，验证 top-up 固定绑定 `ecosystem_pool` controller slot 的 signer allowlist / threshold policy，且 daily restricted grant 只从专用池出账/退款回流 | restricted grant 资金池分层、高权限大池审批与低权限日常发放解耦、运营口径收敛 |

- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-AGC-001 | 第 1 个 agent 认领也必须付费 + 锁 bond + 付 upkeep | 给首个 agent 免费资格，只对第二个起收费 | 免费首槽会直接成为 alt abuse 和零成本囤位入口，且削弱 token sink。 |
| DEC-AGC-002 | 采用 `activation fee + claim bond + upkeep` 三段式 | 只收一次性买断费 | 买断只解决入口，不解决长期占坑与闲置问题。 |
| DEC-AGC-003 | 用 `reputation_tier` 冻结 `1/2/3` 槽位上限，并让后续槽位更贵 | 只靠余额决定能占多少 agent | 只看余额会把组织控制权过度让渡给高资产账号。 |
| DEC-AGC-004 | 强制回收采用“欠费宽限 + 闲置回收”双触发 | 只在玩家手动 release 时释放 agent | 没有系统回收，agent 池会被长期冻结。 |
| DEC-AGC-005 | 先冻结状态机和不可突破边界，绝对价格留给后续平衡调参 | 现在就硬拍绝对 token 数值并直接宣称最终价格 | 当前阶段更缺结构性规则，而不是营销式定价数字。 |
| DEC-AGC-006 | 首轮 producer review 结论为“维持当前 v1 默认值，不新开调参专题” | 在缺少真实 claim 持有分布、回收率和玩家阶段数据前立即改 `slot multiplier / grace / penalty / tier cap` | 当前 runtime / viewer / QA 闭环已证明结构正确，但还没有足够平衡数据支持改价；过早调参只会引入第二套未验证默认值。 |
| DEC-AGC-007 | 在 main token 账本内新增 `restricted starter claim balance`，只作为 `slot-1 claim/upkeep` 的受限 bucket | 直接给首个 agent 免费资格；或直接空投可转账 main token | 免费会破坏“首个 claim 非免费”的规则；可转账空投会直接打开刷号和套现路径。 |
| DEC-AGC-008 | refund / reclaim 必须保留 funding-source provenance，restricted 来源的 bond 退款回 restricted bucket | 所有 bond refund 统一退回 liquid main token | 若 refund 统一回 liquid，玩家可把启动补贴通过 claim/release 洗成可转账资产。 |
| DEC-AGC-009 | restricted bucket 只允许 `slot-1 claim + slot-1 upkeep` 的窄用途，不扩成通用非流通代币 | 让 restricted bucket 覆盖任意 claim、任意 upkeep 甚至其他 main-token 动作 | 窄用途更利于 limited preview / onboarding 启动，不会意外制造第二套通用资产语义。 |
| DEC-AGC-010 | 允许 `slot-1` 使用 `restricted + liquid` 混合支付，并记录 provenance | 要么全部 restricted，要么全部 liquid | 混合支付能避免“restricted 不够一点就完全不能用”的体验断点，同时仍可通过 provenance 保证 refund 不洗钱。 |
| DEC-AGC-011 | 不收窄 `PRD-GAME-011` 对 restricted grant lifecycle / audit 的要求，并保持 starter balance 继续是 `slot-1` 专用、不可转账、可过期可撤销的受限余额 | 把当前实现退化为“只有一个数值 bucket、没有 issuer/expiry/audit 的临时补贴”并据此宣称专题完成 | QA 已证明 bucket 记账与展示闭环正确，但没有正式发放元数据、过期/撤销事件和审计链，就无法证明该补贴受治理、可回收、可复盘。 |
| DEC-AGC-012 | 在 runtime world-state 里把 restricted grant admin 收口到正式 registry，并要求每个 admin account 同时绑定现有 controller signer allowlist policy；issue/revoke 先过 admin gate，再进入 grant 业务校验 | 继续只依赖 runbook 约定 `issuer_id=liveops`；或另起一套与 controller policy 脱钩的独立 admin 配置 | `issuer_id` 只是业务字段，不足以证明操作者具备正式权限；复用既有 governance controller signer policy 能避免平行治理真值，同时把“非 admin”阻断前置到 runtime action 入口。 |
| DEC-AGC-013 | 不开放整份 controller registry 的任意热编辑，只新增一条由 `ecosystem_pool` treasury controller account 签名驱动的 `UpdateRestrictedStarterClaimAdminRegistry` 动作，专门更新 restricted grant admin 子集 | 继续只允许离线 import / 启动注入变更 registry；或把 admin roster 继续绑定到模拟内 passed proposal proposer；或开放完整 controller registry runtime 任意改写 | restricted grant admin roster 属于主链资产治理面，不应继续依赖模拟内 agent/proposal 真值；收窄到单一字段更新，并复用既有 controller slot + signer allowlist / threshold policy，能把 liveops/admin 轮换放回正式钱包治理路径，同时避免把更高风险的 controller policy / treasury slot 编辑一并放开。 |
| DEC-AGC-014 | 为运营补一层薄 CLI `oasis7_liveops_grant_cli`，只封装 `issue/revoke/status` 并继续复用底层 runtime canonical action | 继续让运营手工拼 action JSON；或让 CLI 直接编辑 world snapshot/journal；或顺手开放 admin roster 直改命令 | 问题在于日常操作摩擦过大，不在于底层规则错误；薄 CLI 能降低运营使用成本，同时保持 runtime/state/journal 真值与 controller 治理边界，不引入新旁路。 |
| DEC-AGC-015 | 在正式 CLI 之上补一层仓库脚本 `scripts/oasis7-liveops-grant.sh`，把 world-dir/issuer 缺省、位置参数与常用命令收口，但最终仍只转发到 `oasis7_liveops_grant_cli` | 继续要求运营直接敲长 `cargo run` 命令；或把更多运营逻辑复制进第二套脚本状态机 | 当前痛点已经从“没有 CLI”变成“正式 CLI 仍太长、太像开发命令”；薄 wrapper 可以减少误操作与培训成本，但不能复制第二份业务规则或引入脚本直改 world 的捷径。 |
| DEC-AGC-016 | 允许 `liveops` 这类 restricted grant admin 低权限 slot 在 manifest 中显式声明 `threshold=1`，并以 `1-of-2` signer policy 进入 governance registry；其余 treasury/controller 主槽位继续默认 `2-of-3` | 要么强迫 `liveops` 也走统一 `2-of-3`，抬高运营摩擦；要么直接把所有 controller slot 的 threshold 一起降到 `1` 或 `1-of-2` | `liveops` 只负责不可转账 restricted grant 的日常发放/撤销，权限面明显低于 treasury/controller 主槽位；把 threshold 下调收口到单独 slot，既能降运营 ceremony 成本，又不把更高风险的主槽位一起放宽。 |
| DEC-AGC-017 | 将 daily restricted grant 的 source bucket 从 `ecosystem_pool` 拆成独立 `restricted_starter_claim_liveops_pool`，并只开放一条由 `ecosystem_pool` controller-governed top-up action 负责向该池补款 | 继续让日常发放直接动 `ecosystem_pool`；或把一部分 liquid treasury 先转到 `liveops` 账户；或直接泛化成任意 treasury-to-treasury 转账框架 | 运营需要把高风险大池审批和低权限日常发放明确分层；专用池 + 固定 top-up action 既保留 `ecosystem_pool` 的 `2-of-3` 审批门槛，又不把 `liveops` 变成 liquid treasury 分发者，同时避免当前切片过早扩成通用 bucket 间转账框架。 |
