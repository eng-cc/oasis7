# issue #160 first capability closeout（2026-05-17）

审计轮次: 1

## Meta
- 关联 issue: `#160`
- 关联任务: `close issue 160 first capability gate after PostOnboarding`
- Trace: `.pm/tasks/task_4261de9e42ac422c9ecc63525740fbb9.yaml`
- owner: `runtime_engineer`
- 当前结论: `closeout_ready`

## 本轮要解决的问题
- `#160` 当前不再追“是否已经存在 PostOnboarding 语义”，而是追“active-LLM formal lane 是否真能从 `post_onboarding.establish_first_capability` 推进到首个持续能力里程碑”。
- 2026-04-15 的正式口径仍停留在：
  - `trust gate = hold`
  - `first capability gate = not_run`
- 这次 closeout 只回答“repo-side runtime / LLM contract blocker 是否已经被拆掉，并且 formal lane 是否已经拿到 fresh pass evidence”，不把更宽的 release / liveops 结论混写进来。

## Acceptance criteria mapping

| `#160` 验收点 | 当前仓库真值 | 现行证据 / 来源 | 结论 |
| --- | --- | --- | --- |
| formal active-LLM lane 能稳定进入 `PostOnboarding` 且不再回退到 `first_session_loop` | `900s` formal sample 已稳定保持在 `post_onboarding`，并推进到 `choose_first_expansion_tradeoff / 92%` | `output/playwright/retention-active-llm-formal/issue160-trust-refresh-fix11-capability-window/retention-sample-summary.md`；`.../state-play-samples.jsonl` | `pass` |
| 玩家在 `PostOnboarding` 窗口内继续收到 grounded `goal / progress / blocker / next_step` | formal sample 从 `20%` entry 到 `92%` pre-pause 期间始终保留 canonical gameplay goal/progress surface | `output/playwright/retention-active-llm-formal/issue160-trust-refresh-fix11-capability-window/retention-sample-summary.md` | `pass` |
| 在 `1-3` 次 session 内，formal lane 能到达首个持续能力里程碑或 clear branch handoff | 当前 `900s` 单次 formal sample 已到 `post_onboarding.choose_first_expansion_tradeoff / 92%`，满足旧 capability gate 成功口径 | `output/playwright/retention-active-llm-formal/issue160-trust-refresh-fix11-capability-window/retention-sample-summary.json` | `pass` |
| capability closure 的证据与 trust gate 分开记录，不混写成单层 verdict | summary 同时记录 `trustGateResult=pass` 与 `firstCapabilityResult=pass`，并保留独立 checks | `output/playwright/retention-active-llm-formal/issue160-trust-refresh-fix11-capability-window/retention-sample-summary.json`；`scripts/collect-active-llm-retention-sample.sh` | `pass` |

## 关键修复链

### 1. `environment.current_observation` 不再只回 raw observation
- `crates/oasis7/src/simulator/llm_agent/behavior_runtime_helpers.rs` 现在会在原始 observation 之外显式返回：
  - `current_location_id`
  - `current_location_name`
  - `factory_build_costs_default`
  - `can_build_factory_smelter_mk1_now`
  - `missing_build_prerequisites`
  - `build_ready_summary`
  - `recommended_build_factory_action`
  - `recommended_schedule_recipe_action`
- 这次修改的目的不是“增加更多调试字段”，而是把 active-LLM 在 `20%` 阶段反复声称缺失的 build-safe context 直接下沉成 canonical module output。

### 2. 定向测试已经锁住 build-ready contract
- `crates/oasis7/src/simulator/llm_agent/tests_part3_module_lifecycle.rs`
  - `llm_agent_current_observation_module_exposes_build_ready_context`
- 该回归明确断言 colocated location、`can_build_factory_smelter_mk1_now=true`、空 `missing_build_prerequisites` 与推荐 `build_factory` 模板已可读。

### 3. formal lane 的行为真值已经从 “20% 长停” 切换成 “真实工业推进”
- 短样本 `issue160-trust-refresh-fix10-build-context`
  - 首次出现真实 `BuildFactory` / `ScheduleRecipe` trace
  - 从 `post_onboarding.establish_first_capability / 20%` 推进到 `post_onboarding.stabilize_first_line_after_output / 80%`
- 长窗口样本 `issue160-trust-refresh-fix11-capability-window`
  - `trustGateResult=pass`
  - `firstCapabilityResult=pass`
  - `finalGoalId=post_onboarding.choose_first_expansion_tradeoff`
  - `finalProgressPercent=92`
  - `maxLogicalTime=76`
  - `maxEventSeq=103`

## 当前 contract

### 1. 2026-04-15 的 `hold/not_run` 现在是历史基线，不是当前真值
- `doc/testing/evidence/gameplay-ten-minute-trust-gate-2026-04-09.md` 继续保留为历史 baseline / failure archive。
- 当前 fresh truth 改由本文件与 `issue160-trust-refresh-fix10/11` artefact 负责。

### 2. `#160` 关心的 repo-side blocker 已经收口
- 当前不再是：
  - `post_onboarding.establish_first_capability / 20%` 长停
  - “缺少位置/建造上下文”
  - formal lane 根本不产出工业决策
- 当前 fresh sample 已证明：
  - 模型会发出 `build_factory(factory.smelter.mk1)`
  - 随后会发出 `schedule_recipe(...)`
  - canonical gameplay surface 会推进到第一次扩产取舍阶段

## 作用域边界
- 本 closeout 不声称 broader release readiness、public trial readiness 或 liveops gate 已一起恢复。
- 本 closeout 不把 `internal_playable_alpha_late` 改写成更激进阶段判断。
- 本 closeout 只说明：`#160` 追踪的 `PostOnboarding -> first capability gate` formal-lane closure，已经拿到当前仓库真值与 fresh pass evidence。

## 结论
- issue verdict: `closeout_ready`
- 建议 PR 收口方式: code + evidence PR，PR body 显式 `Closes #160`
- 当前非目标: 不把本次 closeout 扩写成“所有 retention / control-feeling / broader gameplay 问题都已解决”
