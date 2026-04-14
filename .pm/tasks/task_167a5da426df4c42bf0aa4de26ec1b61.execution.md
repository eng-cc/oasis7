# task_167a5da426df4c42bf0aa4de26ec1b61 Execution Log

- task_uid: task_167a5da426df4c42bf0aa4de26ec1b61
- title: Fix industrial gameplay progression regressions after post-onboarding
- owner_role: runtime_engineer
- worktree_hint: oasis7-gameplay-industrial-progression-regressions

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-14 16:26:46 CST / runtime_engineer
- 完成内容: 修复 `runtime_live` 两条确定性 industrial progression regressions。其一，为 `ViewerRuntimeLiveServer` 增加已确认玩家进度基线，不再把 fresh session 的 bootstrap tick 或仅有背景时间推进误判成 `PostOnboarding` 已成立；现在只有在真实控制反馈出现前向增量、或此前已经确认玩家进度后，后续 `blocked` / `completed_no_progress` 才会进入 `post_onboarding.recover_capability`。其二，将 gameplay industrial action 的建厂门槛改成与 runtime `BuildFactory` 真值一致，按 `agent ledger -> world fallback` 判断 `build_factory_smelter_mk1` / `build_factory_assembler_mk1` 是否可执行，不再只看 `world` ledger。
- 完成内容: 补充定向回归测试，覆盖 fresh session + blocked feedback 仍停留 `first_session_loop`、confirmed progress 后 `blocked`/`completed_no_progress` 继续保持 `post_onboarding`、以及 agent ledger 满足 assembler build 前置时 gameplay action 应可用。执行 `cargo fmt --all`，并通过 `env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::tests::snapshot_progress -- --nocapture` 与 `env -u RUSTC_WRAPPER cargo test -p oasis7 viewer::runtime_live::tests::auth_actions -- --nocapture`。
- 遗留事项: 本任务没有改动 `simulator::llm_agent` 的 `recipe_coverage` 语义。复核后确认 simulator 内 `WorldEventKind::RecipeScheduled` 本身就是一次立即结算的完成信号；若 formal active-LLM lane 仍存在 20% 长停，需要继续沿 runtime/provider 决策闭环而不是在这里硬改 coverage 标记。

## 2026-04-14 17:10:34 CST / runtime_engineer
- 完成内容: 根据 `codex-review-snapshot` findings 补上两处遗漏。其一，恢复 `background play` 的独立容错基线：保留 `initial_world_time` 作为“世界自 bootstrap 以来已发生过真实推进”的判断，同时继续把 `confirmed_player_gameplay_progress_time` 仅用于 snapshot stage 机，避免把“玩家已确认进度”和“世界已有历史推进”混成一个信号。其二，收紧 `build_factory_assembler_mk1` 的前台可用性条件，要求完整建造成本必须由同一个 ledger 满足，并把 `structural_frame` 补回门槛检查，和 runtime `BuildFactory` 的整笔成本选择规则保持一致。
- 完成内容: 新增回归覆盖 `runtime_background_play_tolerates_transient_llm_failure_after_confirmed_progress`、`runtime_gameplay_actions_keep_assembler_build_disabled_when_cost_is_split_across_ledgers`，并补跑 `viewer::runtime_live::tests::background_play`，验证“fresh session 不误升阶段”“已有世界进度时 background play 可按预算重试”“split-ledger assembler 成本仍保持禁用”三条边界同时成立。
- 遗留事项: `codex-review-snapshot` 在本环境仍存在长时间只流检查日志、不稳定收口的问题；本次已依据其明确 findings 补完代码，并以 `snapshot_progress`、`auth_actions`、`background_play` 三组 targeted 测试绿灯作为当前收口依据。
