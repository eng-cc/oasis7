# Gameplay control-feeling 与 anti-grind 矩阵（2026-05-23）

审计轮次: 1

## Meta
- 关联专题: `PRD-GAME-014`、`PRD-GAME-015`
- 关联任务: `task_b23cd4919b4c481490777293b556cc70`
- 责任角色: `qa_engineer`
- 协作角色: `producer_system_designer`、`runtime_engineer`、`viewer_engineer`
- 当前结论: `watch`
- 目标: 把 `silent wait without fallback`、`world_activity_only`、`grind_only`、`forced_major_power_dependency` 四类高风险 blocker 固化成正式 QA 矩阵，绑定 repo truth、定向样本入口与 pass/watch/block 判据。

## 最终结论
- 本轮高风险 follow-up 已把前两类 control-feeling blocker 收口到 `pass`：
  - `silent wait without fallback` 现在有 canonical `response_window_class`、`stalled_reason`、`escalation_hint` 与 `fallback_action_*`，QA 不再需要靠 UI 猜“accepted 之后到底是在等、在卡、还是该修”。
  - `world_activity_only` 已继续被现有 `player leverage` rubric 与 playability card 明确阻断，不能再用“世界很活跃”冒充“玩家仍有 meaningful participation”。
- 后两类 mature-world 风险当前被正式提升为 `watch`，而不是假装已经完全通过：
  - `grind_only` 的设计判据、viewer value surface 与 `same_loop_repeat_count` / `grind_only_flag` runtime truth 已经就位；当前仍缺使用这些字段的 fresh mature-world sample 来给出现行 verdict。
  - `forced_major_power_dependency` 的 contract 已冻结为 blocker，runtime truth 已有历史落地；当前仍缺 fresh mature-world sample 去给出“被迫依附 major power”与“可独立 repair/rebuild/pivot”之间的现行 verdict。
- 因此，本矩阵证明的是“高风险 blocker 已有正式 QA 判据与样本锚点”，不等于 `PRD-GAME-012` trust gate、`PRD-GAME-015` mature-world lane 或 broader playability 已整体 `pass`。

## 矩阵

| blocker family | 当前 verdict | formal surface / repo truth | 样本入口 | pass/watch/block 判据 | blocker 签名 |
| --- | --- | --- | --- | --- | --- |
| `silent_wait_without_fallback` | `pass` | `crates/oasis7/src/viewer/runtime_live/gameplay_snapshot.rs` 与 `crates/oasis7/src/simulator/persist.rs` 已发布 `response_window_class`、`stalled_reason`、`escalation_hint`、`fallback_action_id`、`fallback_action_label`；`software_safe` summary 会把它们翻成显式 CTA。 | `viewer::runtime_live::tests::snapshot_progress::compat_snapshot_surfaces_control_feeling_contract_fields_from_gameplay_feedback`、`compat_snapshot_keeps_post_onboarding_no_progress_after_confirmed_progress`、`compat_snapshot_blocks_first_session_when_chain_sync_is_unavailable`、`simulator::tests::persist::snapshot_player_gameplay_execution_state_backfills_from_legacy_fields` | `pass`: accepted/no-progress/blocked 三类状态都能给出 response window + escalation/fallback；`watch`: 只有状态没有 fallback；`block`: accepted 之后仍只剩模糊 `executing`，玩家看不到 repair/refresh/advance 方向。 | `accepted_intent_id` 已存在，但 `response_window_class` 为空，或 `completed_no_progress/blocked` 样本缺 `fallback_action_*` / `escalation_hint`。 |
| `world_activity_only` | `pass` | `doc/playability_test_result/playability_test_card.md`、`doc/testing/prd.md`、`doc/testing/evidence/gameplay-ten-minute-trust-gate-2026-04-09.md` 已要求正式样本显式填写 `player_action`、`world_change_due_to_player`、`player_leverage_score`、`world_activity_only`。 | playability card、trust-gate evidence packet、`snapshot.player_gameplay` / `software_safe` 正式玩家 surface | `pass`: 样本能明确回答“玩家做了什么、世界因此变了什么、是否打开新决策”，且 `world_activity_only=no`；`watch`: 有 player leverage 但仍不稳定；`block`: 只能证明世界在运转或 AI 很忙。 | 样本只写 world delta / 活跃事件 / AI 行为，却说不清玩家动作造成了什么变化，或 `world_activity_only=yes` 仍被拿去支撑 `continue_playing` / lane success。 |
| `grind_only` | `watch` | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md` 与 gameplay 顶层合同已冻结 `anti-grind leverage progression`，`crates/oasis7_viewer/software_safe_src/viewer_feedback_module.js` 与 `main.jsx` 已新增 `Capability Economics`，显式展示 `投入 / 产出 / 新用途 / 修复动作 / 下一步价值`。 | `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`、`npm run test:ui -- software_safe_src/main.test.jsx`、`PRD-GAME-015` anti-grind 条目 | `pass`: checkpoint 能给出新的用途、恢复弹性、议价位或分支价值；`watch`: 已有 value surface，但 fresh mature-world sample 仍未给出当前 route verdict；`block`: 只能继续重复同一工业循环，且系统无法回答“为什么现在值得继续”。 | 连续多个 checkpoint 只剩“产量更高 / 库存更多”，没有 `new use`、`repair elasticity`、`next value` 或 branch worth；或 UI/证据面只能复述 throughput。 |
| `forced_major_power_dependency` | `watch` | `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md` 把强组织非自愿依赖定义为产品失败边界，gameplay 顶层合同要求默认保留 `repair/rebuild/pivot`；runtime truth 已有历史落地，但 fresh mature-world sample 尚未给出当前 verdict。 | `PRD-GAME-015` small-player lane / guardrails 条目、当前 `Capability Economics` surface、`qa-mature-world-small-player-fresh-sample` | `pass`: 样本存在不依附 major power 的继续路径，且 repair/rebuild/pivot 对玩家可见；`watch`: 合同、runtime truth 与部分 viewer surface 已存在，但 fresh sample 仍缺；`block`: 唯一继续方法是加入更大组织、接受外部 sponsor 或放弃当前 lane。 | `major_power_dependency_status=forced`、`requires_major_power_sponsorship=yes` 成为默认前提，或当前样本没有独立 repair/rebuild/pivot path。 |

## 当前建议
- 对 `silent_wait_without_fallback`：
  - 当前可以按 `pass` 处理，但后续任何 regression 只要重新出现“accepted 之后只剩 executing”就应直接打回 blocker。
- 对 `world_activity_only`：
  - 所有 future trust/capability/mature-world 样本继续沿用 `player leverage` rubric；不能因为 viewer 更好看就放弃这个硬门槛。
- 对 `grind_only`：
  - 下一步应由 `qa-mature-world-small-player-fresh-sample` 使用已落地的 `same_loop_repeat_count`、`leverage_class`、`grind_only_flag` runtime truth 复采样；不能用字段存在或 viewer 文案直接升级 verdict。
- 对 `forced_major_power_dependency`：
  - 下一步应在 fresh mature-world sample 中消费已落地的 `major_power_dependency_status` 与 `repair/rebuild/pivot` truth；没有代表性样本时 mature-world `pass` 仍然会被这一项卡住。

## 2026-06-22 runtime truth implementation trace
- 任务: `.pm/tasks/task_96b6823495f44ef39c80f3c8b1a74421.yaml`
- 本轮补齐 canonical `snapshot.player_gameplay` 字段：`small_player_lane_id`、`leverage_class`、`same_loop_repeat_count`、`grind_only_flag`、`major_power_dependency_status`、`recovery_path_kind`、`recovery_path_detail`、`requires_major_power_sponsorship`、`repair_available`、`rebuild_available`、`pivot_available`。
- `same_loop_repeat_count` 不是 viewer 文案推导；runtime `FactoryProductionState` 已记录 `last_completed_recipe_id` 与 `same_recipe_repeat_count`，再由 gameplay snapshot 发布为 small-player lane truth。
- 当前验证证明：
  - legacy `WorldSnapshot.player_gameplay` 缺 small-player lane 字段时会 backfill 为 `unclassified` / `unverified` / `0` / `false`，不破坏旧样本读取。
  - 连续同 recipe 输出会在 snapshot 暴露 `same_loop_repeat_count`，但在尚未达到 mature-world block 阈值前不误报 `grind_only_flag`。
  - 进入 `post_onboarding.choose_first_expansion_tradeoff` 后，snapshot 发布 `leverage_class=regional_specialization_option`、`major_power_dependency_status=independent_path_available` 与 `recovery_path_kind=repair_rebuild_or_pivot`。
- 边界: 本 trace 只说明 runtime/sample truth surface 已开始可测；本文件顶部 `watch` 结论不因本实现自动改为 `pass`。若要升级 verdict，仍需 `qa_engineer` 使用新字段重跑 mature-world small-player lane sample，并归档 pass/watch/block 结论。

## 执行命令
- `rtk env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::tests::snapshot_progress::compat_snapshot_surfaces_control_feeling_contract_fields_from_gameplay_feedback -- --nocapture`
- `rtk env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::tests::snapshot_progress::compat_snapshot_keeps_post_onboarding_no_progress_after_confirmed_progress -- --nocapture`
- `rtk env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::tests::snapshot_progress::compat_snapshot_blocks_first_session_when_chain_sync_is_unavailable -- --nocapture`
- `rtk env -u RUSTC_WRAPPER cargo test -p oasis7 simulator::tests::persist::snapshot_player_gameplay_execution_state_backfills_from_legacy_fields -- --nocapture`
- `rtk node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
- `rtk npm run test:ui -- software_safe_src/main.test.jsx`
- `rtk ./scripts/doc-governance-check.sh`
- `rtk git diff --check`

## 备注
- 本文档的 `watch` 不是模糊措辞，而是正式说明“判据与 runtime truth 已存在，但 fresh mature-world sample 仍不足以给 `pass`”。
- 完成 `qa-mature-world-small-player-fresh-sample` 后应重新生成本矩阵，而不是因字段落地直接沿用或升级当前 `watch` 结论。
