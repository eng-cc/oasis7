# Gameplay Layer War/Governance/Crisis/Meta Closure（生产级设计）

- 对应设计文档: `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.design.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.project.md`

审计轮次: 4


## ROUND-002 主从口径
- 主入口：`doc/game/gameplay/gameplay-top-level-design.prd.md`
- 本文仅维护增量，不重复主文档口径。

## 目标
- 补齐 `README.md` 与 `doc/game/gameplay/gameplay-engineering-architecture.md` 对完整玩法层的最低要求：战争、政治治理、危机、经济覆盖、元进度五类玩法都具备可启动模块与可审计协议原语。
- 在“基础层 / WASM 游戏层”已拆分的前提下，将玩法原语落到 runtime 动作/事件协议、状态机与内建模块启动闭环，形成可测试的最小可玩实现。
- 保持世界不变量（资源守恒、时间单向、Agent 唯一性、治理闭环）不被玩法层绕过。

## 范围

### In Scope
- 扩展 runtime 动作协议：新增联盟、战争、投票、危机、元进度相关动作。
- 扩展 runtime 领域事件：新增对应状态变更事件与拒绝路径。
- 扩展 `WorldState`：持久化联盟关系、战争状态、治理提案投票、危机实例、元进度积分/成就。
- 新增 Gameplay 内建 WASM 模块（5 类）：`war` / `governance` / `crisis` / `economic` / `meta`。
- 新增 Gameplay bootstrap：内建模块注册、激活、`ModuleRole::Gameplay` 与 `GameplayContract` 绑定。
- 新增/更新 runtime 测试：协议行为、状态应用、bootstrap 激活与 mode readiness 覆盖。

### Out of Scope
- 复杂数值平衡（伤害公式调优、治理权重经济学、危机概率策略）和大规模玩法迭代。
- 前端展示改版与新 UI 面板。
- 链上治理合约、跨节点共识细节改动。

## 接口/数据

### Runtime Action 原语（新增）
- `FormAlliance`：提议建立联盟。
- `DeclareWar`：发起战争。
- `CastGovernanceVote`：对治理提案投票。
- `ResolveCrisis`：提交危机应对。
- `GrantMetaProgress`：发放元进度积分与成就。

### Runtime DomainEvent 原语（新增）
- `AllianceFormed`：联盟建立。
- `WarDeclared`：战争建立。
- `GovernanceVoteCast`：投票记录。
- `CrisisResolved`：危机处理结果。
- `MetaProgressGranted`：元进度发放。

### 玩家侧战争后果预览
- `DeclareWar` / 调整强度 / 暂缓 / 先谈判动作必须能派生 `war_declaration_quote` / `conflict_outcome_preview`。
- 最小字段：`actor_alliance_id`、`target_alliance_id`、`action_kind`、`intensity`、`recommended_intensity_band`、`war_duration_ticks`、`aggressor_score_estimate`、`defender_score_estimate`、`likely_winner_before_action`、`victory_margin_estimate`、`conflict_window_blocked_until`、`reentry_cooldown_or_active_conflict_blocker`、`expected_narrative_or_module_reward`、`settlement_risk`、`alternative_action`、`recommended_war_action`、`why_this_war_is_worth_or_risky`。
- Edge case: 若玩家准备宣战或调整强度，但看不到预计胜负、持续时间、冲突占用、结算风险、推荐强度或替代行动理由，标记为 `war_declaration_quote_missing`。
- Acceptance: 战争原语不应只在 `WarDeclared` / `WarConcluded` 事件中解释；玩家提交宣战前应能判断当前时机、目标、强度、胜算、结算后果和替代路径。
- Non-goal: 本补充不改变战争动作、结算公式、持续时间、胜负判定、runtime ABI 或内建模块启动闭环；只冻结玩家侧后果可读性。

### 状态模型（新增）
- `alliances: BTreeMap<String, AllianceState>`
- `wars: BTreeMap<String, WarState>`
- `governance_votes: BTreeMap<String, GovernanceVoteState>`
- `crises: BTreeMap<String, CrisisState>`
- `meta_progress: BTreeMap<String, MetaProgressState>`

### Gameplay 内建模块目标
- 每个模块均声明 `role=Gameplay`，并带 `abi_contract.gameplay`：
  - `kind`: `war | governance | crisis | economic | meta`
  - `game_modes`: 至少覆盖 `sandbox`
- 通过 bootstrap 后，`gameplay_mode_readiness("sandbox")` 达到五类覆盖。

## 里程碑
- GLC-1：协议与状态模型闭环（action -> domain event -> state）。
- GLC-2：内建 Gameplay WASM 模块与 bootstrap 闭环。
- GLC-3：测试与文档收口（required/full 档位分别覆盖协议与 wasm 启动）。

## 风险
- 新增协议原语后，旧模块订阅标签可能需要扩展，若漏配会导致观测缺口。
- Gameplay bootstrap 若未接入现有场景入口，将出现“模块存在但默认不开启”的落差。
- 状态字段新增会放大快照兼容风险，需要依赖 `serde(default)` 保证回放兼容。
- 玩法原语先实现 MVP 语义，后续数值策略调整频繁，需持续回归测试避免行为漂移。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
