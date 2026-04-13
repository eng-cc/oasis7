# oasis7 主链 Token 初始分配与早期贡献奖励口径（2026-03-22）设计

- 对应需求文档: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`

## 1. 设计定位
定义主链 Token 创世分配、创世绝对发行量、控制边界、低流通门禁与早期贡献奖励的统一设计，让 Token 发行从一开始就是“可审计的战略配置”，而不是“事后补解释的营销行为”。

## 2. 设计结构
- 创世分配层：冻结 `10000 bps` 分配表、bucket 命名、控制主体与锁仓方式。
- 绝对总量层：冻结当前 `main_token_config.initial_supply = 10,000,000,000 OC`，把比例表落成可复核的绝对金额。
- 控制边界层：区分项目战略控制、协议奖励池、单人直接受益和外部流通。
- 奖励执行层：把 early contributor reward 约束为 evidence-based 审核与受控发放。
- 审计门禁层：对比例求和、个人上限、低流通和禁语边界做 QA/治理复核。

## 3. 关键接口 / 入口
- `Action::InitializeMainTokenGenesis`
- `Action::ClaimMainTokenVesting`
- `Action::DistributeMainTokenTreasury`
- `WorldState.main_token_genesis_buckets`
- `WorldState.main_token_treasury_balances`

## 3.1 TIGR-1 创世参数表（草案）
- 实现约束：
  - `InitializeMainTokenGenesis` 当前只会把 bucket 分配写入 recipient account 的 `vested_balance`，不会直接初始化 `main_token_treasury_balances`。
  - 因此本表中的 `protocol:*` recipient 全部是 custody account 名称草案，不等于 runtime treasury bucket 名称。
  - 当前暂按 `genesis_epoch=0`、`1 epoch ~= 1 day` 进行锁仓换算；若最终链上 epoch 节奏不同，需保持相同自然时间重新换算。

| bucket_id | ratio_bps | recipient | controller | start_epoch | cliff_epochs | linear_unlock_epochs | recommended_claim_cadence | 说明 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `team_long_term_vesting` | `2000` | `protocol:team-core-vesting` | team multisig | `0` | `365` | `1095` | quarterly | 核心团队主锁仓盘 |
| `early_contributor_reward_reserve` | `1500` | `protocol:early-contributor-reward` | reward multisig + producer approval | `0` | `0` | `3650` | batch-by-batch only | 早期贡献奖励储备，控制首年最大可 claim 容量 |
| `node_service_genesis_custody` | `2000` | `protocol:node-service-custody` | protocol governance / node committee | `0` | `180` | `1825` | governance batch | 创世节点服务储备，和 post-genesis `node_service_reward_pool` 区分 |
| `staking_genesis_custody` | `1500` | `protocol:staking-custody` | protocol governance | `0` | `180` | `1825` | governance batch | 创世质押储备，和 post-genesis `staking_reward_pool` 区分 |
| `ecosystem_governance_reserve` | `1500` | `protocol:ecosystem-governance` | ecosystem governance multisig | `0` | `90` | `1460` | quarterly | grant / ecosystem 计划储备 |
| `security_reserve_emergency` | `1000` | `protocol:security-council-reserve` | security council multisig | `0` | `0` | `0` | emergency only | 安全事故与协议防御储备 |
| `foundation_ops_reserve` | `500` | `protocol:foundation-ops` | ops multisig | `0` | `90` | `730` | monthly or quarterly | 基础设施与运营成本储备 |

## 3.2 控制边界汇总
- 项目战略控制：`team_long_term_vesting + early_contributor_reward_reserve + security_reserve_emergency + foundation_ops_reserve = 5000 bps`
- 协议长期储备：`node_service_genesis_custody + staking_genesis_custody = 3500 bps`
- 生态/治理储备：`ecosystem_governance_reserve = 1500 bps`
- 创始人个人直持：不单列独立大 bucket；如需个人受益，必须内嵌在 `team_long_term_vesting` 受益人表内并继续受 `500~1000 bps` 目标区间与 `1500 bps` 硬上限约束。

## 3.2A 创世绝对总量冻结（TIGR-9）
- 当前冻结值：`main_token_config.initial_supply = 10,000,000,000 OC`
- 对应首年非团队外部释放边界：
  - 目标：`100,000,000~200,000,000 OC`
  - 硬上限：`500,000,000 OC`
- 对应 bucket 绝对分配额：
| bucket_id | ratio_bps | allocated_amount_at_10b |
| --- | --- | --- |
| `team_long_term_vesting` | `2000` | `2,000,000,000 OC` |
| `early_contributor_reward_reserve` | `1500` | `1,500,000,000 OC` |
| `node_service_genesis_custody` | `2000` | `2,000,000,000 OC` |
| `staking_genesis_custody` | `1500` | `1,500,000,000 OC` |
| `ecosystem_governance_reserve` | `1500` | `1,500,000,000 OC` |
| `security_reserve_emergency` | `1000` | `1,000,000,000 OC` |
| `foundation_ops_reserve` | `500` | `500,000,000 OC` |
- 该口径下 `allocated_amount` 全为整数，runtime rounding remainder 为 `0`；执行清单继续沿用 runtime 真值算法，但当前冻结值不会触发补差额分配。

## 3.3 TIGR-4 制作人执行路径决策
- 当前 limited preview 期间，`early_contributor_reward_reserve` 保持为独立的 reward multisig / producer approval 执行路径，不并入 `ecosystem_pool`。
- 原因-1：`InitializeMainTokenGenesis` 当前只把分配写入 recipient account `vested_balance`，创世时并不会直接生成 treasury bucket 余额。
- 原因-2：limited preview 需要更强的低流通与低承诺纪律，独立储备更容易把“贡献奖励”与“生态 grant”分开管理。
- 原因-3：`TIGR-3` 已把奖励口径固定为 contribution-based review，如果现在并入 `ecosystem_pool`，会放大对外对 grant / reward / airdrop 的语义混淆。
- 重审条件：至少完成 1~2 轮真实贡献奖励台账、审批记录与复盘，再由新专题决定是否迁移到 fully on-chain 的治理分发路径。

## 3.4 TIGR-5 正式执行清单
- 目标：把 `TIGR-1` 的逻辑草案收成 pre-mint freeze sheet，明确每个 bucket 的 `recipient_slot_id / controller_slot_id / signer_policy / runtime_target / freeze_status`。
- runtime 真值：
  - `Action::InitializeMainTokenGenesis` 输入的是 `MainTokenGenesisAllocationPlan`。
  - runtime 会先按 `floor(initial_supply * ratio_bps / 10000)` 计算 `allocated_amount`，再将 remainder 按 `ratio_bps` 降序、`bucket_id` 升序逐个补 1。
  - 生成的 `MainTokenGenesisAllocationBucketState` 会写入 `main_token_genesis_buckets`，并把 recipient 对应账户的 `vested_balance` 初始化为聚合后的已分配金额。
- 当前冻结值下的执行含义：
  - `initial_supply = 10,000,000,000 OC`
  - 由于 7 个 bucket 的 `ratio_bps` 在该口径下都能整除，当前 `remainder = 0`。
- 冻结原则：
  - 逻辑参数、slot registry、签名规则和 rounding 规则现在冻结。
  - 创世绝对发行量 `10,000,000,000 OC` 与 7 个 bucket 的绝对分配额现在冻结。
  - 真实链上 `recipient_account_id` 与 multisig 地址允许后补，但在补齐前 freeze status 只能是 `ready_pending_address_binding`。
  - 最终 mint 前仍必须以 QA checklist 输出 `pass`，不能把 formal sheet 当作最终放行。

## 4. 约束与边界
- 创世分配总和必须为 `10000 bps`。
- 项目战略控制目标为 `5000 bps`，单人直接受益硬上限为 `1500 bps`。
- 创世液态流通硬上限为 `500 bps`。
- 早期奖励只能按贡献证据发放，不能用 `play-to-earn` 叙事替代。
- `TIGR-1` 参数表所有 bucket 的 `genesis_liquid` 默认都为 `0`；若后续要形成 liquid，必须通过 claim 动作显式发生并留下审计记录。

## 5. 设计演进计划
- 先冻结比例和控制边界。
- 再冻结当前创世绝对发行量 `10,000,000,000 OC` 与 7 个 bucket 绝对分配额。
- 再落实具体 bucket/account/vesting 参数表与审计 checklist。
- 再输出 slot-based 正式执行清单，固定 runtime 落点与 rounding 规则。
- limited preview 期间先按独立 reward reserve 多签执行。
- 后续先绑定真实地址并跑 QA 最终审计，再决定是否进入 mint 准备。
- 更后续根据真实奖励轮次与治理成熟度，再决定 early contributor reserve 的长期执行载体。
