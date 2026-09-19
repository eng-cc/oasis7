# Gameplay Agent 认领经济合同设计

- 对应需求文档: `doc/game/gameplay/gameplay-agent-claim-economy-contract.prd.md`
- 产品承诺入口：[`Agent 所有权与持续经营`](../../product/world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-001)
- 可变执行状态: 对应 GitHub Project task 与 issue evidence comments

审计轮次: 2

<a id="public-claim-boundary"></a>
## Public claim boundary

本设计文件只负责 claim 状态机、成本、资金来源和 provenance 的专业设计，不拥有发行状态或公开 claim。其消费合同是玩家接入与发行模块的 [`REQ-ENTRY-CLAIM-003`](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#req-entry-claim-003) 与 [`四层口径分离`](../../product/player-entry-distribution/release-communications-and-public-claims.prd.md#public-claim-four-layer-model)。

- 当前事实（current fact）：设计中明确的是状态和约束的设计事实；它不能证明实现已经部署、某个账户已经获准、某个渠道已发布或当前可公开体验。
- 产品目标（product goal）：让首个 claim 有明确成本，让受限 starter balance 只用于 `slot-1`，并让回收、退款和失败边界可审计、可向玩家解释。
- 假设（assumption）：受控运营窗口会提供与设计一致的 runtime、viewer、QA 和 operator evidence；若任一条件不成立，不能继续沿用设计文案作为当前状态。
- 证据边界（evidence limit）：设计只支持规则解释和候选 claim 的限制说明，不支持 public launch、production settlement、一般账户资格、无条件可用性或资产价值承诺。
- 可发布公开 claim（releasable public claim）：只有产品 PRD、根 README、同一证据窗口和对应 [`restricted grant LiveOps runbook`](gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md#public-claim-boundary) 一致时，才可引用本设计的受限规则；否则必须标记为 draft、blocked 或 stale。

## 1. 设计目标
- 把 agent 认领从“谁先点到谁拿到”的弱规则，提升为一条有成本、有维护、有回收的正式 gameplay 经济链路。
- 保持 `Agent 唯一性` 与资源守恒，不把 claim 机制做成可绕过 runtime 审计的侧门。
- 让 `runtime_engineer`、`viewer_engineer`、`qa_engineer` 对同一套 claim 状态机和字段工作。
- 让 limited preview / allowlist / QA seed 可以给首个 claim 提供受限启动资金，而不把启动补贴变成可转账资产。

## 2. 状态机

`unclaimed -> quote_ready -> claimed_active -> upkeep_grace -> forced_reclaimed -> unclaimed`

补充分支：
- `claimed_active -> released -> unclaimed`
- `claimed_active -> inactive_reclaim_candidate -> forced_reclaimed -> unclaimed`

关键约束：
- 任一 agent 任一时刻只允许 1 个 `claim_owner_id`。
- 首个 claim 也必须走完整成本链：`activation fee + claim bond + upkeep`。
- `restricted starter claim balance` 只改变 `slot-1` 的资金来源，不改变 canonical 成本，也不能旁路 `slot-2/3` 成本。
- `upkeep_grace` 与 `inactive_reclaim_candidate` 都必须带可见倒计时，不能只靠后台静默清退。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [`REQ-WR-AOS-001`](../../product/world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-001) | 首个 Agent 认领必须展示非零承诺包，并保持成本、资金来源与后续回收可审计。 | [`DES-CLAIM-ECONOMY-001`](#des-claim-economy-001) | `producer_system_designer` / `runtime_engineer` | 不证明具体实现、部署或公开 claim。 |

<a id="des-claim-economy-001"></a>
### DES-CLAIM-ECONOMY-001：认领成本与资金来源承接

本设计的状态机、成本模型与 restricted funding 边界承接 `REQ-WR-AOS-001` 的非零承诺包；资金来源、回收和退款 provenance 必须保持可审计，但本表不把设计写成实现或发布证明。

## 3. 成本模型
- 三段式：
  - `activation fee`: 非退款，立即拆到 burn / treasury。
  - `claim bond`: 锁定后可在 release / reclaim 时按规则退款或 slash。
  - `upkeep`: 每个 epoch 结算一次，持续表达“占有这个 agent 就要持续承担成本”。
- 报价必须把持续成本换算成玩家可读 runway：
  - `upfront cost`: activation fee + claim bond + first upkeep。
  - `recurring upkeep`: 每个 epoch 的持续成本。
  - `runway`: 扣除 upfront 后，当前 eligible balance 还能覆盖多少个完整 upkeep epoch。
  - `reclaim risk`: 低 runway 时预计进入 grace / forced reclaim 的风险提示。
  - `recommended top-up or wait`: 若 runway 低于阈值，提示先补足余额、等待收入或暂缓扩张。
- runtime v1 暂定默认值（用于 canonical 实现与测试，不等于最终平衡定价承诺）：
  - `base activation fee = 100`
  - `base claim bond = 200`
  - `base upkeep = 25`
  - `activation fee burn split = 50%`
- 槽位曲线：
  - `slot-1 multiplier = 1.0`
  - `slot-2 multiplier = 1.5`
  - `slot-3 multiplier = 2.0`
- 声誉上限：
  - `tier-0 cap = 1`
  - `tier-1 cap = 2`
  - `tier-2+ cap = 3`
  - runtime v1 暂按 `reputation_score < 10 / >= 10 / >= 25` 映射到 `tier-0 / tier-1 / tier-2+`，后续由平衡专题复核。
- 受限 bucket：
  - `restricted starter claim balance` 是 main token 账本中的受限 bucket，不是第二种代币。
  - 允许消费范围：`slot-1 claim upfront cost` 与 `slot-1 upkeep`。
  - 禁止消费范围：`TransferMainToken`、公开转账 API、explorer 导出的可转账金额、`slot-2/3 claim` 与其它 main-token 动作。
  - 资金优先级：`slot-1` 先花 restricted，再补 liquid；`slot-2/3` 只花 liquid。
  - 发放来源：`allowlist / onboarding / qa_seed / liveops_campaign`；每笔发放都必须带 `issuance_reason / issuer_id / expires_at_epoch`。

## 4. 退款与 Provenance
- claim 时要显式记录：
  - `upfront_restricted_spent_amount`
  - `upfront_liquid_spent_amount`
  - `claim_bond_locked_restricted_amount`
  - `claim_bond_locked_liquid_amount`
- release / forced reclaim 时：
  - 先按 canonical 规则结清 arrears 与 penalty。
  - 再把剩余 bond 按 provenance 拆成 `restricted refund` 与 `liquid refund`。
  - restricted 来源的 refund 只能回 restricted bucket，不能洗成 liquid。
- 若 upfront 为 mixed funding，则 activation fee / upkeep 属于已消耗成本，不参与退款；只有 bond 部分需要保留来源拆分。

## 5. 回收与退款
- 主动释放：
  - `release_cooldown_epochs = 3`
  - 满足 cooldown 且无欠费时，按 provenance 退回剩余 bond。
- 欠费回收：
  - `grace_epochs = 2`
  - 逾期未补足则强制回收。
- 闲置回收：
  - 连续 `7` 个 epoch 无有效控制推进，进入 `inactive_reclaim_candidate`
  - 连续 `10` 个 epoch 仍无恢复，执行强制回收。
- 惩罚：
  - `forced_reclaim_penalty_bps = 2000`
  - 先扣未付 upkeep，再对剩余 bond 扣 penalty。

## 6. Runtime / Viewer / QA 边界
- `runtime_engineer`
  - 负责 claim 状态机、受限 bucket 账本、原子扣费、epoch 结算、refund / slash provenance 和事件。
- `viewer_engineer`
  - 负责 quote、restricted/liquid 余额拆分、funding mix、upkeep deadline、cooldown、idle risk、cap 阻断原因、refund 预估与 upkeep runway 的表达。
  - Claim quote 最小条必须包含 `upfront cost / recurring upkeep / runway / reclaim risk / recommended top-up or wait`，避免玩家把可支付 upfront 误读为可长期持有。
- `qa_engineer`
  - 负责并发争抢、受限余额发放、欠费、闲置、多槽位、refund provenance、transfer guard、审计字段和 UI/API parity 的 required/full 验收。
- `producer_system_designer`
  - 负责成本曲线、tier cap、宽限与回收边界，以及 starter balance 的用途、额度与过期策略。
- `liveops_community`
  - 负责 allowlist / campaign 发放策略、回收/停用策略与受控测试口径回流。

## 7. 设计边界
- 这不是现实货币付费功能，也不是永久产权出售。
- 这不是 agent 市场或 NFT 化系统。
- v1 先冻结规则结构和默认边界，不在本轮拍死最终绝对价格。
- 这也不是“通用不可转账代币”系统；restricted bucket 只服务于首个 agent 启动。

## 8. 演进顺序
- 先落文档与任务拆解，冻结“首个也不免费”的正式口径。
- 再落 runtime canonical 字段与记账事件。
- 然后落受限 bucket、transfer guard 与 refund provenance。
- 最后补 Viewer / pure API 表达与 QA abuse suite，再决定是否进入新一轮平衡调参。

## 9. 首轮平衡复核结论
- `TASK-GAMEPLAY-AGC-005`（2026-03-27）当前结论：继续维持 v1 默认值，不新开调参专题。
- 维持项：
  - `slot multiplier = 1.0 / 1.5 / 2.0`
  - `grace_epochs = 2`
  - `forced_reclaim_penalty_bps = 2000`
  - `tier cap = 1 / 2 / 3`
- 维持理由：
  - runtime / viewer / QA 三条闭环都已通过，说明当前更缺“真实持有行为样本”，而不是“再造一轮默认值”。
  - 当前成本曲线仍满足本专题最重要的结构边界：首个 claim 非免费、额外槽位单调更贵、欠费与闲置都能回收、refund/slash 可审计。
  - 在还没有真实 claim 分布、平均持有时长、grace 命中率、forced reclaim 占比之前，提前改参数只会稀释对当前默认值的验证意义。
- 后续只有在以下任一条件成立时，才重新开调参专题：
  - `liveops_community` 回流显示 claim churn、grace 命中率或 idle reclaim 占比异常。
  - `qa_engineer` 发现当前 cap / penalty 造成稳定的玩法退化或反滥用失效。
  - producer 拿到首轮真实组织扩张数据，能证明 `slot-2/3` 或 `tier cap` 已经系统性过轻或过重。

## 10. 本轮 Reopening 结论
- 2026-03-29：由于当前文档把 claim 资金来源写死为 `liquid main token`，与受控测试阶段“允许发受限启动资金、但不打开可转账空投”的产品结论冲突，`PRD-GAME-011` 重新进入 `in_progress`。
- reopening 范围：
  - 新增 `restricted starter claim balance` bucket。
  - 新增 `slot-1` 专用消费范围、transfer guard 与 refund provenance。
  - 不改变 `activation fee + claim bond + upkeep` 三段式，也不改“首个 claim 非免费”的根规则。

## 11. 验证设计与可追溯性

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [`REQ-WR-AOS-001`](../../product/world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-001) | [`DES-CLAIM-ECONOMY-001`](#des-claim-economy-001) | 验证首个 claim 的非零成本、受限资金用途与回收边界保持同一规则链。 | [`agent claim runtime tests`](../../../crates/oasis7/src/runtime/tests/agent_claims.rs) | GitHub task evidence / QA result | 不证明生产部署、公开资格或最终价格平衡。 |
