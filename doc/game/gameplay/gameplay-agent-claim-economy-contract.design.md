# Gameplay Agent 认领经济合同设计

- 对应需求文档: [`Gameplay Agent 认领经济合同`](gameplay-agent-claim-economy-contract.prd.md)
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
- 首个经济 `ClaimAgent` 也必须走完整成本链：`activation fee + claim bond + upkeep`。首次 `RegisterAgent` / 会话绑定不创建该状态机中的经济 claim，不能仅凭绑定显示 bond 或 upkeep 义务。
- `restricted starter claim balance` 只改变 `slot-1` 的资金来源，不改变 canonical 成本，也不能旁路 `slot-2/3` 成本。
- `upkeep_grace` 与 `inactive_reclaim_candidate` 都必须带可见倒计时，不能只靠后台静默清退。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [`REQ-WR-AOS-001`](../../product/world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-001) | 首个经济 Agent 认领必须展示非零承诺包，并保持成本、资金来源与后续回收可审计。 | [`DES-CLAIM-ECONOMY-001`](#des-claim-economy-001) | `producer_system_designer` / `runtime_engineer` | 不证明具体实现、部署或公开 claim。 |
| [REQ-AGC-CHAT-001](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-001) | 首聊资源边界按 OR 条件披露，并保留余额耗尽后记录仍有效与其他独立 gate 的区别。 | [DES-CLAIM-CHAT-002](#des-claim-chat-002) | `runtime_engineer` / `viewer_engineer` | 不改 OC 额度、领取资格、账本或现有 chat gate。 |
| [REQ-AGC-CHAT-002](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-002) | 注册/绑定不等于经济认领；权威延后说明按实际经济 claim 生成。 | [DES-CLAIM-CHAT-003](#des-claim-chat-003) | `runtime_engineer` / `gameplay_designer` | 不据此减免费用、清除历史未决义务或开放免费经济 claim。 |
| [REQ-AGC-CHAT-003](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-003) | 中英文、Viewer/pure API 保留同一含义；旧版或缺失义务说明保守回退。 | [DES-CLAIM-CHAT-004](#des-claim-chat-004) | `viewer_engineer` / `qa_engineer` | 不要求新增协议字段或把显示状态持久化为世界状态。 |

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

### 6.1 首次绑定与经济认领的组件边界

既有 [gameplay action 映射](../../../crates/oasis7/src/viewer/gameplay_actions.rs) 将 `claim_first_agent` 交给 `RegisterAgent`，将 `claim_agent` 交给 `ClaimAgent`。首次绑定只建立可操作 Agent；首聊维护说明需要另外读取该 Agent 作为 claimer 的真实经济 claim。

| 组件 | 本次实现职责 | 依赖与边界 |
| --- | --- | --- |
| [claim_snapshot.rs](../../../crates/oasis7/src/viewer/runtime_live/claim_snapshot.rs) | 读取调用方传入的 `WorldState` 和 `primary_agent_id`，保留现有 preview 生成条件，按真实 `owned_claims` 选择维护说明；输入覆盖范围见下文。 | 当前身份、控制权和动作 blocker 仍由既有控制面处理；本函数不接收旧 DTO、不写账本或执行动作。 |
| [first_chat_unlock_preview.jsx](../../../crates/oasis7_viewer/software_safe_src/first_chat_unlock_preview.jsx) | 本地化新的资源说明和两类权威维护文案；旧版无条件维护文案或缺失义务说明回退为 unknown。 | 只消费六字段 preview，不从字段缺省值、经济报价或英文句子反推 ownership 或动作资格。 |
| [viewer_app.jsx](../../../crates/oasis7_viewer/software_safe_src/viewer_app.jsx) / 既有 API 投影入口 | 沿用当前预览传递、展示和可用动作合同；同一预览在 Viewer / API 中保留相同含义。 | 不为此次文案修正新增授权、身份、快照版本或缓存失效机制。 |

本地 runtime 测试使用完整 `WorldState`：`main_token_balances[primary_agent_id].liquid_balance` 沿用缺少账户项为零的既有语义；`starter_oc_claims.contains_key(primary_agent_id)` 提供领取记录存在性；`agent_claims` 中 `claim_owner_id == primary_agent_id` 的记录组成现有 `owned_claims`。`next_claim_quote` 是未来选择，不能证明当前经济认领已经生效。

service 来源需单独限定：[world_service/projection.rs](../../../crates/oasis7/src/world_service/projection.rs) 的 allowlist 保留 `agent_claims` 与 `starter_oc_claims`，但不发布 `main_token_balances`；[world_service_link.rs](../../../crates/oasis7/src/viewer/runtime_live/world_service_link.rs) 用该裁剪状态重建 runtime world，再由 [control_blocking.rs](../../../crates/oasis7/src/viewer/runtime_live/control_blocking.rs) 调用本投影。经济 claim 事实仍可用于维护说明，但 service 的真实正 liquid 余额且无领取记录可能因余额未投影而按零值生成预览。这是既有余额输入缺口，本次不修改字段公开范围或 gate；本地回归与同一预览的中英文/API 显示一致性不能证明真实 service gate 已正确。

本次范围是权威生产者文案与 Viewer 的旧文案兼容，不新增 DTO 字段存在性追踪、count/list 协调、gate 三态或资料新鲜度检测。本地状态与当前 service 投影均保留 `agent_claims`；按当前 claimer 筛选的完整结果为空时，才支持没有 owned claim 的结论。经过默认值补齐的旧 preview DTO 不提供这种证据，Viewer 不使用它来生成“没有经济认领”的新结论。

<a id="des-claim-chat-002"></a>
### 6.2 首聊 gate 与资源边界投影

承接 [REQ-AGC-CHAT-001](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-001)。现有生成条件保持 `liquid_balance == 0 && !starter_oc_claims.contains_key(primary_agent_id)`，只修正说明，不改变 chat gate。下表以余额与领取记录均完整的权威输入为前提，本次 gate 回归在本地 runtime 状态上验证；service 输入限制见 6.1 节。

| 余额与记录完整的权威输入 | `first_chat_unlock_preview` | 必须表达的边界 |
| --- | --- | --- |
| liquid OC 为零且没有 starter OC 记录 | 生成现有六字段预览 | 资源和延后字段都解释完整 OR 条件；`recommended_unlock_action` 仍标识 `claim_starter_oc`。实际可执行性由既有动作面判断。 |
| liquid OC 大于零，或已有 starter OC 记录 | 不生成该解锁预览 | 任一条件都足以通过这一道 gate；已有记录时余额用尽不重开必领提示，也不产生重复领取资格。 |
| 调用方传入的 Agent 在世界中不存在 | 沿用既有 `None` 结果 | 不为缺失 Agent 生成首聊解锁预览。 |

`resource_boundary` 独立包含一次性首聊解锁与初始 liquid OC 授予、`liquid OC > 0 OR starter_oc_claims record exists`、记录保留时余额用尽仍通过，以及与经济 slot-1 claim/upkeep funding 分离。`defer_effect` 复用同一 gate 解释，避免两个字段产生不同含义。通过 starter OC gate 不代表其他授权、Agent 控制权或 provider 前提已经通过。

兼容旧文案时，Viewer 只补全上述稳定规则说明，不用它重新计算当前 gate 或发布可用动作。

<a id="des-claim-chat-003"></a>
### 6.3 经济 claim 事实与延后后果

承接 [REQ-AGC-CHAT-002](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-002)。`chat_purpose` 统一使用当前有控制权的 Agent；经济维护责任由独立的真实 claim 事实决定。

| 输入与证据 | `defer_effect` 规则 | 判定所在层 |
| --- | --- | --- |
| 完整经济 claim 集合中，当前 claimer 的 `owned_claims` 为空 | 暂缓不改变当前 Agent 绑定，也不新增经济认领或 upkeep 义务。 | Rust 权威快照生产者；不从未来报价或其他 Agent 的 claim 推导已有义务。 |
| 完整经济 claim 集合中，当前 claimer 的 `owned_claims` 非空 | 暂缓不改变已有经济认领，其当前维护规则继续适用。 | Rust 权威快照生产者；不额外收费、延长 runway 或清除原有义务。 |
| Viewer 仅收到旧版无条件 upkeep 文案，或 `defer_effect` 缺失、为空、只有空白 | 经济认领义务尚未确认，请刷新权威状态；暂缓不创建或清除义务。 | Viewer 兼容回退；该旧文案或缺失说明无法证明有 claim 或无 claim。 |

两类权威文案及 unknown 回退都附同一 OR gate 解释。所有变化限于披露，查看和延后不触发 claim、release、费用或账本动作。既有经济规则仍随世界时间运行；当前经济集合为空不证明所有历史未决义务已经结清。

<a id="des-claim-chat-004"></a>
### 6.4 显示、语言与兼容处理

承接 [REQ-AGC-CHAT-003](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-003)。沿用六字段 preview、现有动作面与精确字符串本地化映射。新生产者直接输出准确英文；Viewer 对同一含义显示对应中文。

| 场景 | 中文含义 | English meaning |
| --- | --- | --- |
| 资源边界 | 初始 OC 是一次性首聊解锁与初始可用 OC；可用 OC 大于零或已有领取记录即可通过这一道门。领取记录仍在时，余额用尽不会重新锁定此门；它与经济认领/维护资金分开。 | Starter OC provides a one-time first-chat unlock and initial liquid OC. A positive liquid OC balance or an existing starter OC record passes this gate, even after the credited balance is spent. Economic claim and upkeep funding are separate. |
| 已确认无当前经济 claim | 暂缓不改变当前 Agent 绑定，也不会新增经济认领或维护义务。 | Deferring leaves your Agent binding unchanged and creates no economic claim or upkeep obligation. |
| 已有经济 claim | 暂缓不改变已有经济认领，其现行维护规则继续适用。 | Deferring leaves your existing economic claims and their current upkeep rules unchanged. |
| 旧版或缺失义务说明 | 当前经济认领义务尚未确认，请刷新权威状态；暂缓不会创建或清除义务。 | Current economic claim obligations are not confirmed. Refresh authoritative state; deferring creates or clears no obligation. |

兼容转换只识别受影响的旧版精确文案：旧目的说明改为当前有控制权的 Agent，旧版或缺失资源说明补全静态 gate 规则，旧版或缺失义务说明改为 unknown。其他非空来源文本保持原义；缺少翻译时显示来源文本，不退回另一类经济义务句子。此转换不是从文案推断世界状态。

组件响应调用方提供的预览变化，替换上一条说明；身份选择和过期快照处理仍由原有调用方负责。更新该组件源码后通过既有构建生成 Viewer 产物，显示和翻译不提交领取请求或其他 world action。

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
| [`REQ-WR-AOS-001`](../../product/world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-001) | [`DES-CLAIM-ECONOMY-001`](#des-claim-economy-001) | 验证首个 claim 的非零成本、受限资金用途与回收边界保持同一规则链。 | [`agent claim runtime tests`](../../../crates/oasis7/src/runtime/tests/agent_claims.rs) | PR 与实际验证记录 / QA result | 不证明生产部署、公开资格或最终价格平衡。 |
| [REQ-AGC-CHAT-001](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-001) | [DES-CLAIM-CHAT-002](#des-claim-chat-002) | OR gate、余额耗尽后记录保留，以及资源字段独立披露。 | [首聊 snapshot 回归](../../../crates/oasis7/src/viewer/runtime_live/tests/snapshot_first_chat_unlock_preview.rs) 的 `compat_snapshot_projects_first_chat_unlock_preview_only_for_zero_liquid_without_starter_oc`；同一 source HEAD 的本地 required Rust 层。通过条件：零余额且无记录才有预览，两个文案字段均说明 OR 和余额用尽；这些组合区分实际 gate 与旧文案。 | PR 记录命令、HEAD、本地输入来源与实际测试 verdict。 | 不证明 service 余额投影与真实 gate 正确、provider 可用、重复领取资格或已完成真实对话。 |
| [REQ-AGC-CHAT-002](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-002) | [DES-CLAIM-CHAT-003](#des-claim-chat-003) | 实际注册与经济认领的义务区分。 | [首聊 snapshot 回归](../../../crates/oasis7/src/viewer/runtime_live/tests/snapshot_first_chat_unlock_preview.rs) 的 `first_chat_unlock_preview_distinguishes_registration_from_economic_claims`；同一 source HEAD 的 action → snapshot 层。通过条件：真实 RegisterAgent 无经济 claim，真实 ClaimAgent 有 owner/upkeep，其他 Agent 的 claim 不污染文案；同时检查世界记录和说明，避免仅有字符串自证。 | PR 记录世界状态断言与实际测试 verdict。 | 不改变经济状态机或证明所有历史未决义务已结清。 |
| [REQ-AGC-CHAT-003](gameplay-agent-claim-economy-contract.prd.md#req-agc-chat-003) | [DES-CLAIM-CHAT-004](#des-claim-chat-004) | 两种语言、旧版与缺失义务说明、预览更新保持相同含义。 | [首聊 Viewer 测试](../../../crates/oasis7_viewer/software_safe_src/first_chat_unlock_preview.test.jsx)；同一 source HEAD 的组件层并构建生成产物。通过条件：EN/ZH 对应、旧版和缺失说明为 unknown、无翻译保留来源、输入更新替换旧说明；覆盖精确字符串映射最易遗漏的兼容分支。 | PR 记录组件测试 verdict、构建结果与证据范围。 | 不证明新鲜度检测、全场景可玩性、真实留存或发行就绪。 |

<a id="first-chat-validation-matrix"></a>
### 11.2 首聊披露的最小验收矩阵

选择同一待验收 source HEAD，执行本节对应的本地权威投影与显示测试，实际 verdict 留在 PR 验证记录。本设计不因列出场景而宣称测试已经通过，也不把文档检查当作运行证据。

| 样本 | 权威前置或组件输入 | 必须观察的结果 | 主要验证层 |
| --- | --- | --- | --- |
| 首次注册、尚无经济认领 | 通过 `claim_first_agent` 映射的真实 `RegisterAgent` 创建 Agent；经济 claim 集合为空；零 liquid、无领取记录 | 有首聊价值预览；资源边界完整；不声称已有 upkeep；未来 `next_claim_quote` 不当作已完成认领。 | runtime action → snapshot |
| 已有真实经济认领及其他 Agent | 真实 `ClaimAgent` 成功，owner/target/维护状态可核对；零 liquid、无领取记录 | claimer 的延后说明保留既有维护规则；世界中有另一 Agent 的经济 claim 不使当前无 owned claim 的 Agent 获得维护责任文案。 | runtime action → snapshot |
| OR 正分支与领取后余额用尽 | 非零 liquid 且无记录；另测记录存在时 liquid 从正数变为零 | 均无必领预览；领取记录保留时不重新要求解锁。 | 状态序列 → snapshot |
| 旧版或缺失义务说明 | 旧版无条件维护句子；或 defer 字段为缺失、null、空白 | 义务说明为 unknown，不虚构已有 claim，也不改写成没有责任；资源说明保留完整 OR 条件。 | Viewer 兼容显示 |
| 中英文与无翻译回退 | 无 claim、有 claim、unknown 与无对应翻译的来源说明 | 两种语言的条件和义务一致；没有翻译时显示可读来源文本；保留六个字段标签及建议动作含义。 | Viewer 组件与生成产物 |
| 预览输入更新 | 调用方依次传入无 claim、有 claim、缺失义务说明的预览 | 组件替换旧说明，并按新输入显示对应含义；不残留上一条 claim 结论。 | Viewer 响应式显示 |

经济义务断言须与实际世界记录核对；只断言旧字符串存在或组件存在不足以验收。本地完整状态测试和同一预览显示测试不覆盖 6.1 节的真实 service 余额输入缺口。此矩阵不增加 DTO 完整性解析、gate 三态、快照失效检测或新权限流程，也不以 Issue 关闭代替实际运行结果。
