# Gameplay Layer Lifecycle Rules Closure（生产级设计）设计

- 对应需求文档: `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.project.md`

## 1. 设计定位
定义 Gameplay 专题设计，统一玩法循环、运行时/治理边界、玩家反馈与发布验收口径。

## 2. 设计结构
- 玩法循环层：明确微循环、中循环、长循环或发布闭环中的关键行为链。
- 运行接线层：把玩法规则与 runtime、wasm、治理、观测或 longrun 门禁对齐。
- 治理预览层：把提案生命周期、法定人数、通过阈值和票权影响转成玩家提交前可读的 outcome preview。
- 反馈验收层：定义玩家可感知反馈、平衡目标与质量门禁。
- 演进收口层：将生产化、发布差距或模块化切片纳入可追踪计划。

## 3. 关键接口 / 入口
- gameplay 规则/模块入口
- runtime / wasm / governance / viewer 接线点
- `governance_vote_quote` / `proposal_outcome_preview`: `proposal_id`、`proposal_topic`、`actor_id`、`action_kind`、`closes_at_tick`、`ticks_remaining`、`current_quorum_weight`、`required_quorum_weight`、`current_pass_bps`、`required_pass_bps`、`actor_vote_weight`、`vote_swing_potential`、`likely_outcome_before_action/after_action`、`affected_rule_or_priority`、`world_change_if_passed`、`cost_or_cooldown_if_failed`、`recommended_governance_action`、`why_this_vote_matters`
- 玩法反馈与平衡指标
- gameplay 回归与发布门禁

## 4. 约束与边界
- 玩法设计需与运行时边界、治理规则和测试口径保持一致。
- 生产化收口不得牺牲核心可玩性与反馈清晰度。
- `governance_vote_quote_missing` 只表示玩家无法判断当前票局、自己票权影响、通过后变化或失败/过期代价；不得被解释为要重平衡阈值、投票窗口、票权公式、冷却或新增复杂政治模拟。
- 不在单个专题中扩张到全量玩法体系重写。

## 5. 设计演进计划
- 先冻结该专题的玩法闭环与关键指标。
- 再补 runtime/治理/观测接线与验证口径。
- 最后以发布或长期回归为门禁完成收口。
