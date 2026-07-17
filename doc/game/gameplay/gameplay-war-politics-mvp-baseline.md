# 战争与政治机制最小可行数值基线（MVP）

审计轮次: 4

## 目标

- 为 Gameplay 战争与政治玩法提供可执行、可验证、可回归的首轮数值基线。
- 明确“成本 / 收益 / 冷却”三类约束，避免进入单一最优策略或无反馈状态。
- 对齐当前 runtime + m5 gameplay wasm 实现，减少设计与实现口径偏差。

## 范围

### In Scope
- 战争：宣战门槛、强度范围、结算时长、胜负评分、重入冷却约束。
- 政治：提案窗口、法定人数、通过阈值、投票重投规则、提案重试节奏。
- 与实现的常量映射和测试入口。

### Out of Scope
- 大规模平衡性调优（多赛季、跨服差异化参数）。
- 联盟外交声望系统与跨服政治机制。
- UI 层可视化和引导文案优化。

## 接口/数据

- 动作入口：
  - `Action::DeclareWar`
  - `Action::OpenGovernanceProposal`
  - `Action::CastGovernanceVote`
- 事件出口：
  - `DomainEvent::WarDeclared`
  - `DomainEvent::WarConcluded`
  - `DomainEvent::GovernanceProposalOpened`
  - `DomainEvent::GovernanceVoteCast`
  - `DomainEvent::GovernanceProposalFinalized`

## 里程碑

- B1：基线冻结（本文档）。
- B2：按基线运行 1 周回放，输出偏差报告（冲突频次、治理参与度）。
- B3：仅在偏差超过阈值时进入第二轮调参（保持小步迭代）。

## 风险

- 若战强度奖励过高，可能诱发“无脑高强度宣战”。
- 若治理窗口过长，政治反馈滞后导致玩家体感“决策无效”。
- 若提案阈值过高，治理参与会收敛到少数头部玩家。

---

## 1. 战争数值基线（MVP）

| 维度 | 基线值 | 成本/收益/冷却含义 | 实现锚点 |
|---|---|---|---|
| 宣战强度 `intensity` | `1..=10` | 强度仅提高进攻评分，同时延长冲突窗口；战争内核不保证结算收益随强度增长 | `crates/oasis7/src/runtime/world/event_processing.rs` |
| 战争持续时长 | `6 + 2 * intensity` ticks | 形成显式投入成本（占用冲突窗口） | `crates/oasis7/src/runtime/state.rs`、`crates/oasis7_builtin_wasm_modules/m5_gameplay_war_core/src/lib.rs` |
| 战争结算评分 | `aggressor_score = members*10 + intensity`；`defender_score = members*10` | 进攻方获得强度加成；防守方依赖组织规模 | `crates/oasis7/src/runtime/world/gameplay_loop.rs` |
| 胜负判定 | `aggressor_score >= defender_score` 时进攻方胜 | 平分时进攻方胜，鼓励主动冲突但保留成员规模价值 | `crates/oasis7/src/runtime/world/gameplay_loop.rs` |
| 同对联盟重入 | 同一联盟对在 active 期间不可重复宣战 | 作为首轮“冲突冷却”约束，避免刷宣战事件 | `crates/oasis7/src/runtime/world/event_processing.rs` |

### 1.1 战争推荐操作区间（用于评审）

当前评分和平分规则下，`recommended_intensity` 表示“达到目标胜负结果的最低强度”，而不是通用强度带：

| 进攻方与防守方成员差 | 最低胜利强度 | 推荐动作 |
|---|---:|---|
| 进攻方不少于防守方 | `1` | 如果仍值得宣战，使用 `1` |
| 进攻方少 1 人 | `10` | 只有 `10` 能追平评分并利用平分时进攻方胜出的规则 |
| 进攻方少 2 人或以上 | 不可达 | 先补强、谈判或等待，不推荐宣战 |

- 高于最低胜利强度只会增加冲突占用时间，除非具体 gameplay 模块同时返回可验证的增量奖励。
- 若连续 3 天 `conflict_freq_100ticks > 8`，先检查最低胜利强度的重复宣战是否构成刷取路径；本基线不通过调整推荐带代替数值平衡。

### 1.2 战争收益说明

- 当前实现中的核心收益是状态与叙事收益：`winner_alliance_id`、战报摘要、冲突历史沉淀。
- 额外经济/元进度收益通过 gameplay 模块 directive 注入，不在战争内核硬编码。
- 预览中的 `expected_narrative_or_module_reward` 必须区分战争内核保证的胜负/历史结果与可选模块奖励；未获得模块回执时，不得把可选奖励显示为保证收益。

### 1.3 当前平衡边界

当前基线存在“最低必胜强度”的支配策略：同规模进攻方用强度 `1` 即可稳定胜出，而弱两人或以上时没有强度翻盘路径。未来若受控重启战争主线，必须通过独立平衡任务决定是否增加强度递增收益/失败损失，或修改评分与平分规则。本轮仅纠正文档口径，不完成数值重平衡。

### 1.4 宣战后果预览

当玩家准备 `DeclareWar`、调整强度、暂缓或先谈判时，玩家侧必须能读取 `war_declaration_quote` / `conflict_outcome_preview`，用于判断当前宣战是否值得、强度是否合适、冲突窗口会被占用多久，以及结算后果是否优于替代行动。

最小字段：
- `actor_alliance_id`
- `target_alliance_id`
- `action_kind`: `declare_war` / `change_intensity` / `defer` / `negotiate_first`
- `intensity`
- `recommended_intensity_band`：按当前评分与平分规则推导的最低胜利强度；无可达强度时必须返回“补强/谈判/等待”而不是虚构推荐带
- `war_duration_ticks`
- `aggressor_score_estimate`
- `defender_score_estimate`
- `likely_winner_before_action`
- `victory_margin_estimate`
- `conflict_window_blocked_until`
- `reentry_cooldown_or_active_conflict_blocker`
- `expected_narrative_or_module_reward`
- `settlement_risk`
- `alternative_action`: `negotiate` / `recruit` / `wait` / `governance_proposal`
- `recommended_war_action`
- `why_this_war_is_worth_or_risky`

Edge case: 当玩家准备 `DeclareWar` 或调整强度，但看不到预计胜负、持续时间、冲突占用、结算风险、推荐强度或替代行动理由时，标记为 `war_declaration_quote_missing`。

Acceptance: 玩家宣战前至少能看懂“为什么现在打、打谁、用多大强度、会持续多久、大概能不能赢、输了/赢了会发生什么、是否应该先谈判或补强”。

Non-goal: 本预览不重平衡战争强度、评分公式、持续时长或胜负判定，不新增完整战斗模拟、战术地图、外交谈判系统或复杂联盟声望系统，不改变 `DeclareWar` / `WarDeclared` / `WarConcluded` runtime ABI，也不把战争收益硬编码进经济或元进度结算。

---

## 2. 政治数值基线（MVP）

| 维度 | 基线值 | 成本/收益/冷却含义 | 实现锚点 |
|---|---|---|---|
| 投票窗口 `voting_window_ticks` | `1..=1440` | 窗口越长，参与覆盖更高，但反馈延迟更大 | `crates/oasis7/src/runtime/world/event_processing.rs` |
| 通过阈值 `pass_threshold_bps` | `5000..=10000` | 阈值越高，提案稳定性越高，但通过成本更高 | `crates/oasis7/src/runtime/world/event_processing.rs` |
| 法定人数 `quorum_weight` | `> 0` | 避免“零参与通过”，保证最小治理成本 | `crates/oasis7/src/runtime/world/event_processing.rs` |
| 选项数 | 至少 2 个唯一选项 | 防止伪提案，确保存在真实选择 | `crates/oasis7/src/runtime/world/event_processing.rs` |
| 重投规则 | 同一投票者可重投，后票覆盖前票 | 允许策略更新，但保持单人单权重口径 | `crates/oasis7/src/runtime/state.rs`、`crates/oasis7_builtin_wasm_modules/m5_gameplay_governance_council/src/lib.rs` |
| 过期处理 | `now > closes_at` 的投票拒绝；到期自动结算 | 显式治理冷却边界，避免无限拖延 | `crates/oasis7/src/runtime/world/event_processing.rs`、`crates/oasis7/src/runtime/world/gameplay_loop.rs` |

### 2.1 政治推荐参数模板（首轮）

- 常规经济提案：
  - `voting_window_ticks = 24..72`
  - `quorum_weight >= 3`
  - `pass_threshold_bps = 6000`
- 制度/战争相关提案：
  - `voting_window_ticks = 48..120`
  - `quorum_weight >= 5`
  - `pass_threshold_bps = 7000..8000`

### 2.2 提案冷却建议（流程约束）

- 由于 `proposal_key` 全局唯一，建议采用 `proposal.<topic>.<epoch>` 命名。
- 同主题提案建议最短重提间隔 `>= 48 ticks`（流程约束，后续可实现为 runtime 硬约束）。

### 2.3 治理投票结果预览

当玩家准备发起提案、投票、改票、等待或暂缓时，玩家侧必须能读取 `governance_vote_quote` / `proposal_outcome_preview`，用于判断当前票局、自己行动是否关键、以及通过/失败会带来什么变化。

最小字段：
- `proposal_id`
- `proposal_topic`
- `actor_id`
- `action_kind`: `open_proposal` / `cast_vote` / `change_vote` / `wait` / `defer`
- `closes_at_tick`
- `ticks_remaining`
- `current_quorum_weight`
- `required_quorum_weight`
- `current_pass_bps`
- `required_pass_bps`
- `actor_vote_weight`
- `vote_swing_potential`
- `likely_outcome_before_action`
- `likely_outcome_after_action`
- `affected_rule_or_priority`
- `world_change_if_passed`
- `cost_or_cooldown_if_failed`
- `recommended_governance_action`
- `why_this_vote_matters`

Edge case: 当玩家准备发起提案、投票或改票，但看不到剩余时间、quorum/pass 缺口、自己票权影响、通过后的世界变化或失败/过期代价时，标记为 `governance_vote_quote_missing`。

Acceptance: 玩家在发起提案或投票前，至少能看懂“这项提案还差多少能成立/通过、我的动作会不会改变结果、通过后具体改变什么、失败或暂缓有什么代价、现在应该投/改票/拉人/等待的理由”。

Non-goal: 本预览不重平衡治理阈值、投票窗口、票权公式或冷却时间，不新增复杂议会 UI、拉票系统、外交谈判系统或全局政治模拟，不改变 `OpenGovernanceProposal` / `CastGovernanceVote` runtime ABI，也不把治理结果硬绑定到经济定价或社会声誉分。

---

## 3. 回归测试入口

- 基线动作协议与状态闭环：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol:: -- --nocapture`
- 模块驱动结算链路：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol::step_with_modules_applies_gameplay_directive_emits_to_domain_events -- --nocapture`
- 全量 required-tier 门禁：
  - `./scripts/ci-tests.sh required`
