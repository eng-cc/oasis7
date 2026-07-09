# Gameplay Layer War/Governance/Crisis/Meta Closure（生产级设计）设计

- 对应需求文档: `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.project.md`

## 1. 设计定位
定义 Gameplay 专题设计，统一玩法循环、运行时/治理边界、玩家反馈与发布验收口径。

## 2. 设计结构
- 玩法循环层：明确微循环、中循环、长循环或发布闭环中的关键行为链。
- 运行接线层：把玩法规则与 runtime、wasm、治理、观测或 longrun 门禁对齐。
- 战争预览层：把宣战强度、持续时长、结算评分和重入约束转成玩家提交前可读的胜算、占用窗口和结算风险。
- 反馈验收层：定义玩家可感知反馈、平衡目标与质量门禁。
- 演进收口层：将生产化、发布差距或模块化切片纳入可追踪计划。

## 3. 关键接口 / 入口
- gameplay 规则/模块入口
- runtime / wasm / governance / viewer 接线点
- `war_declaration_quote` / `conflict_outcome_preview`: `actor_alliance_id`、`target_alliance_id`、`action_kind`、`intensity`、`recommended_intensity_band`、`war_duration_ticks`、`aggressor_score_estimate`、`defender_score_estimate`、`likely_winner_before_action`、`victory_margin_estimate`、`conflict_window_blocked_until`、`reentry_cooldown_or_active_conflict_blocker`、`expected_narrative_or_module_reward`、`settlement_risk`、`alternative_action`、`recommended_war_action`、`why_this_war_is_worth_or_risky`
- 玩法反馈与平衡指标
- gameplay 回归与发布门禁

## 4. 约束与边界
- 玩法设计需与运行时边界、治理规则和测试口径保持一致。
- 生产化收口不得牺牲核心可玩性与反馈清晰度。
- `war_declaration_quote_missing` 只表示玩家无法判断宣战胜算、冲突窗口、结算风险、推荐强度或替代行动；不得被解释为要重平衡战争公式、增加战术地图、外交谈判系统或复杂联盟声望系统。
- 不在单个专题中扩张到全量玩法体系重写。

## 5. 设计演进计划
- 先冻结该专题的玩法闭环与关键指标。
- 再补 runtime/治理/观测接线与验证口径。
- 最后以发布或长期回归为门禁完成收口。
