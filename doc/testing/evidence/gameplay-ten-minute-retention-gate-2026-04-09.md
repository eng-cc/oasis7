# Gameplay 10-minute retention gate verdict (2026-04-09)

审计轮次: 2

## Meta
- 关联专题: `PRD-GAME-012`
- 关联任务: `TASK-GAME-065`
- 责任角色: `qa_engineer`
- 裁决角色: `producer_system_designer`
- 当前结论: `hold`
- 目标: 在当前 10 分钟留存修复切片下，区分 active-LLM formal lane 与 debug/probe lane，重新核对 `software_safe` floor，并输出 producer 可直接采纳的 `continue_playing / hold` 结论。

## Lane boundary
- active-LLM formal lane:
  - 当前正式入口必须走 launcher stack + active LLM access。
  - 本轮正式样本统一通过 `./scripts/collect-active-llm-retention-sample.sh` 执行，并复用当前 task worktree 中从 `main` 复制过来的 real-provider `config.toml`，该文件仅用于本轮 local real-provider 复跑，不纳入提交。
  - 该 lane 只有在最小控制 floor 通过后，才允许累计 10 分钟 retention 样本。
- debug / probe lane:
  - `--no-llm` / observer-only / pure API parity / PostOnboarding headed rerun 只用于中循环和 UI 语义排障，不形成正式留存放行。
  - 代表证据:
    - `doc/playability_test_result/card_2026_03_19_09_40_56.md`
    - `doc/playability_test_result/card_2026_03_19_13_25_04.md`
    - `doc/playability_test_result/card_2026_03_22_15_56_13.md`

## Floor recovery recap
- failed fresh slice run id: `20260409-172309`
- failed blocker signature:
  - `http request failed: request timed out after 10000ms: error sending request for url (https://api.letai.run/v1/responses)`
  - player-facing 结果是 formal lane 停在 `first_session_loop.create_first_world_feedback`，`lastControlFeedback.effect` 明确写成 `gameplay blocked before requested advance completed`
- real-main-config rerun:
  - run id: `20260409-225330`
  - artifact root: `output/playwright/viewer-software-safe-step-real-provider/20260409-225330/`
  - direct result:
    - `renderMode=software_safe`
    - `stepAccepted=true`
    - `selectedAgentVisible=true`
    - `logicalTimeAdvanced=true`
    - `feedbackStage=completed_advanced`
  - gate implication:
    - formal lane 首个 `step` 已恢复 first-step progress，说明当前真实 provider + repo-root config 组合下，`software_safe` 最小控制 floor 已从 runtime timeout blocker 恢复。

## Formal 10-minute evidence

### Qualified 10-minute samples

| 样本 | run id | 工件目录 | 结果卡 | 关键事实 | QA 结论 |
| --- | --- | --- | --- | --- | --- |
| A | `20260410-125829` | `output/playwright/retention-active-llm-formal/active-llm-retention-20260410-125829/` | `doc/playability_test_result/card_2026_04_10_12_58_29.md` | `playDurationMs=600000`，`reachedPostOnboarding=true`，`maxLogicalTime=43`，`finalGoalId=post_onboarding.establish_first_capability`，`finalProgressPercent=20` | formal lane 稳定连通，但 10 分钟内没有把 `post_onboarding.establish_first_capability` 从 `20%` 推进到可持续能力闭环；`continue_playing` 不成立。 |
| B | `20260410-132858` | `output/playwright/retention-active-llm-formal/active-llm-retention-20260410-132858/` | `doc/playability_test_result/card_2026_04_10_13_28_58.md` | 先到 `post_onboarding / 20%`，随后在样本中后段 UI 语义回退到 `first_session_loop.create_first_world_feedback / 0%`，且 `logicalTime=22`、`eventSeq=7` 在余下样本保持不变 | formal lane 不只是“20% 不涨”，还出现阶段语义回退并伴随时间冻结，属于更强的 retention blocker。 |
| C | `20260410-134323` | `output/playwright/retention-active-llm-formal/active-llm-retention-20260410-134323/` | `doc/playability_test_result/card_2026_04_10_13_43_23.md` | 先到 `post_onboarding / 20%`，随后在样本前中段即回退到 `first_session_loop.create_first_world_feedback / 0%`，且 `logicalTime=13`、`eventSeq=6` 在余下样本保持不变 | B 的回退冻结签名在另一独立 10 分钟样本中再次复现，说明当前 `hold` 不是单次偶发噪音。 |

### Supplemental shorter samples
- `20260410-111006`
  - `playDurationMs=300000`
  - 已到 `post_onboarding.establish_first_capability`
  - `finalProgressPercent=20`
  - 说明 5 分钟窗口内也只看到“到达 20%”而不是“完成首个可持续能力”
- `20260410-111714`
  - `playDurationMs=300000`
  - 已到 `post_onboarding.establish_first_capability`
  - `finalProgressPercent=20`
- `20260410-112553`
  - `playDurationMs=300000`
  - 已到 `post_onboarding.establish_first_capability`
  - `finalProgressPercent=20`

### Excluded / non-formal artifacts
- `20260410-131151`
  - 接近 10 分钟但被人工中断，缺 final summary/final screenshot/final packaging，只能作旁证。
- `20260410-113507`
  - 三条并行长跑在同一秒共用了 `startup-20260410-113507`，artifact 互相污染，不作为正式 gate 证据。

## Gate summary
- formal lane sample count:
  - qualified 10-minute samples: `3 / 3`
  - supplemental 300s samples: `3`
  - excluded samples: `2`
- `software_safe` floor verdict: `pass`
- headed Web/UI retention verdict: `hold`
- QA gate input: `hold`
- exact blocker signature:
  - 样本 A 证明当前阻断不再是 provider timeout 或首步 floor 崩溃，而是 formal lane 在 `post_onboarding.establish_first_capability` 长时间卡在 `20%`
  - 样本 B/C 进一步证明存在更强阻断：同一正式 lane 会在进入 `post_onboarding / 20%` 后回退到 `first_session_loop.create_first_world_feedback / 0%`，且 `logicalTime/eventSeq` 冻结不再增长

## Producer verdict
- producer decision: `hold`
- rationale:
  - `PRD-GAME-012` 的当前 formal lane 不再受 `Responses API` 10 秒 timeout 控制 floor 阻断，但这不等于 retention gate 可放行。
  - 三条正式 10 分钟样本里，没有任何一条证明玩家能在 10 分钟窗口内建立“首个可持续能力”。
  - 其中两条样本已经出现阶段语义回退 + 逻辑时间冻结，说明当前 blocker 不是单纯 pacing 偏慢，而是正式 progression/retention 仍未闭环。

## Required follow-up before re-open
- `producer_system_designer` 需要把当前 gate 从“已恢复到 watch”更新为“floor pass，但 active-LLM retention hold”，停止对外延伸 `continue_playing` 口径。
- `runtime_engineer` / `viewer_engineer` 需要先解释并修复这两个签名中的至少一个，再重新申请正式复验：
  - `post_onboarding.establish_first_capability` 长时间停在 `20%`
  - `post_onboarding -> first_session_loop` 的阶段语义回退伴随 `logicalTime/eventSeq` 冻结
- 继续保留 debug/probe lane 的 `--no-llm` 工业与 UI 语义回归，但这些样本不得再作为 formal retention 结论。
