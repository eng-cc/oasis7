# task_319c1fc645b04dd185f3afb45dcd00ee Execution Log

- task_uid: task_319c1fc645b04dd185f3afb45dcd00ee
- title: Investigate post_onboarding establish_first_capability 20 percent stall in active-LLM retention lane
- owner_role: runtime_engineer
- worktree_hint: oasis7-game-post-onboarding-20-percent-stall

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-04-12 14:23:57 CST / runtime_engineer
- 完成内容: 对照 `TASK-GAME-065` 的正式样本 A/B/C、前一轮 `task_7bdbbf9839c74c9eb7bb8c7c161e87de` 快照修复与 `task_fb967ddaadde459786e286b484bc4b0c` freeze hardening 后，确认 `post_onboarding.establish_first_capability / 20%` 仍有第三条独立签名，而且是 active-LLM industrial schema 三层一起漂移：`llm_agent` prompt/runtime helper 仍停留在 assembler-only `factory_kind/recipe_id`，`recipe_coverage` 只跟踪 assembler 三条配方，而 shadow kernel `recipe_plan()` 甚至不会接受 `recipe.smelter.*`；但当前 `PostOnboarding` canonical 目标链与 `runtime_live` gameplay actions 已切换到 smelter-first bootstrap，导致 world time 可以继续前进，但 LLM 决策空间会长期拿不到、或在 shadow decision path 中直接拒掉，`factory.smelter.mk1` / `recipe.smelter.*` 这条首个可持续能力链。
- 完成内容: 已在 `crates/oasis7/src/simulator/llm_agent.rs`、`crates/oasis7/src/simulator/llm_agent/behavior_runtime_helpers.rs`、`crates/oasis7/src/simulator/llm_agent/prompt_assembly.rs`、`crates/oasis7/src/simulator/kernel/actions{,_impl_part3}.rs` 与对应测试中对齐 smelter-first 工业口径，补齐 `recipe.smelter.* -> factory.smelter.mk1` 的 required-factory/default-cost fallback、tracked recipe coverage、prompt/bootstrap/failure-recovery 文案，以及 shadow kernel 对 smelter recipe 的接受路径，避免 active-LLM formal lane 继续因为旧的 assembler-only / unsupported smelter 组合而稳定空转在 `20%`。
- 完成内容: 已执行定向回归 `env -u RUSTC_WRAPPER cargo test -p oasis7 prompt_assembly_includes_harvest_max_amount_cap -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 smelter_recipes_map_to_smelter_factory_kind -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 smelter_recipes_expose_default_cost_fallbacks -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_hard_switches_schedule_recipe_to_next_uncovered_recipe -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_rewrites_wait_ticks_to_sustained_schedule_after_full_recipe_coverage -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_rewrites_wait_to_recovery_action_after_full_recipe_coverage -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_user_prompt_includes_recipe_coverage_summary -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 schedule_recipe_accepts_smelter_recipe_on_smelter_factory -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 schedule_recipe_rejects_incompatible_factory_kind -- --nocapture`，均通过。
- 遗留事项: 本任务解释并修复了 20% 长停的一条确定性 schema drift 来源，但尚未重跑 fresh active-LLM formal retention 样本；在新的 10 分钟正式样本证明 `post_onboarding.establish_first_capability / 20%` 不再长停之前，`PRD-GAME-012` 仍必须保持 `hold`。

## 2026-04-12 14:50:42 CST / runtime_engineer
- 完成内容: 基于第一次 `./scripts/pm/codex-review-snapshot.sh` 的 P1 finding，已把 `schedule_recipe` coverage hard-switch 从“跨所有 tracked recipes 取下一个未覆盖项”收敛为“仅在当前已知 `factory_kind` 的未覆盖配方内切换”，避免 assembler 工厂上的重复配方被错误改写成 smelter 配方后立刻因工厂不兼容而被拒；并新增回归 `llm_agent_keeps_hard_switch_within_current_factory_kind`，保留 smelter 场景回归 `llm_agent_hard_switches_schedule_recipe_to_next_uncovered_recipe`。
- 完成内容: 一并移除了 `build_factory` 在缺失 `factory_kind` 时回落到 `factory.assembler.mk1` 的旧 parser 默认值，改为直接拒收，并新增 `llm_agent_rejects_build_factory_without_factory_kind` 锁定；这样 smelter-first 提示面不会再被缺参输出偷偷拉回 assembler 旧默认。
- 完成内容: 已重跑 `env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_hard_switches_schedule_recipe_to_next_uncovered_recipe -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_keeps_hard_switch_within_current_factory_kind -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 llm_agent_rejects_build_factory_without_factory_kind -- --nocapture`、`env -u RUSTC_WRAPPER cargo fmt --check`，并确认 `git diff --check` 通过。
- 遗留事项: 仍需等待基于当前最新代码的 snapshot review 最终结果；若 clean，则继续 signal / task done / commit。formal retention fresh run 仍不在本任务内自动判定通过。
