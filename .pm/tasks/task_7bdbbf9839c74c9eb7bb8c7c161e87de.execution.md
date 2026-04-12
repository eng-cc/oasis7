# task_7bdbbf9839c74c9eb7bb8c7c161e87de Execution Log

- task_uid: task_7bdbbf9839c74c9eb7bb8c7c161e87de
- title: Fix post-onboarding gameplay snapshot regression after blocked retention runs
- owner_role: runtime_engineer
- worktree_hint: null

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-12 12:15:00 CST / runtime_engineer
- 完成内容: 复核 `PRD-GAME-012` retention hold 的 runtime 侧签名后，确认 `post_onboarding -> first_session_loop` 回退至少有一部分是 `build_player_gameplay_snapshot` 的阶段判定缺口：快照此前把“是否已经越过首轮可操作地板”过度绑定到最近一次 feedback delta，导致 prior progress 之后只要后续反馈变成 `blocked` / `completed_no_progress` 且 delta 为 0，就会错误掉回 `first_session_loop`，把 retention blocker 伪装成“重新回到新手态”。
- 完成内容: 已在 `crates/oasis7/src/viewer/runtime_live/gameplay_snapshot.rs` 将“是否已确认进入首轮可操作之后阶段”的判定从“仅依赖最近一次正向 feedback”扩成“最近一次正向 feedback 或 world time 已前进”；这样 prior progress 后的失败样本会继续保留 `post_onboarding` 及显式 blocker 语义，但 fresh first-step `blocked` / `completed_no_progress` 仍会维持在真实的 `first_session_loop` floor failure。
- 完成内容: 已在 `crates/oasis7/src/viewer/runtime_live/tests/snapshot_progress.rs` 新增 `compat_snapshot_keeps_post_onboarding_blocked_after_prior_progress` 与 `compat_snapshot_keeps_post_onboarding_no_progress_after_prior_progress` 两个回归测试，并与既有 `compat_snapshot_promotes_to_post_onboarding_after_control_feedback`、`compat_snapshot_exposes_player_gameplay_snapshot` 一起通过，确认 runtime-live 快照既不会在 prior progress 后错误掉回新手态，也不会把 fresh floor failure 误报成 post-onboarding blocker。
- 遗留事项: 本任务只修复了 formal retention 样本里的“阶段误回退”口径问题，没有消除 `post_onboarding.establish_first_capability / 20%` 长停，也没有消除真实 `logicalTime/eventSeq` 冻结；`PRD-GAME-012` 的 active-LLM retention gate 仍应保持 `hold`，下一切片需要继续解释 world advance 为什么会停住。

## 2026-04-12 12:25:00 CST / runtime_engineer
- 完成内容: `./scripts/pm/codex-review-snapshot.sh` 二次复核时发现一个真实 P1：runtime live bootstrap 为注册初始 agent 会先执行一次内部 `world.step()`，因此简单使用 `state.time > 0` 会把 fresh LLM gameplay 会话错误判成已经越过 `first_session_loop`。这会让新会话直接显示 `post_onboarding.*` 目标，属于新的首局引导回归。
- 完成内容: 已把 `ViewerRuntimeLiveServer` 的 bootstrap 基线时间显式保存为 `initial_world_time`，并把快照门槛改成“最近一次 feedback 真正产生 delta，或当前 `state.time` 超过 bootstrap 基线”；这样既忽略初始化 tick，又保留 prior progress 之后的 blocked/no-progress 样本语义。
- 完成内容: 已新增 `compat_snapshot_keeps_first_session_loop_for_fresh_llm_session` 回归测试，确保 fresh LLM-enabled runtime-live 会话仍停留在 `first_session_loop.create_first_world_feedback`，不会因 bootstrap tick 被误提升到 `PostOnboarding`。
- 遗留事项: 仍需重新跑 snapshot review 并在 clean verdict 后再提交本任务单 commit；本切片仍未处理 formal retention 的真实停滞/冻结根因。
