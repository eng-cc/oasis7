# Gameplay 10-minute retention gate verdict (2026-04-09)

审计轮次: 1

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
  - 本轮 fresh 复采命令: `./scripts/viewer-software-safe-step-regression.sh --out-dir output/playwright/viewer-software-safe-step-retention --startup-timeout 600 --viewer-port 4473 --web-bind 127.0.0.1:5311 --live-bind 127.0.0.1:5323 --chain-status-bind 127.0.0.1:5421`
  - 该 lane 只有在最小控制 floor 通过后，才允许继续累计 10 分钟 retention 样本。
- debug / probe lane:
  - `--no-llm` / observer-only / pure API parity / PostOnboarding headed rerun 只用于中循环和 UI 语义排障，不形成正式留存放行。
  - 代表证据:
    - `doc/playability_test_result/card_2026_03_19_09_40_56.md`
    - `doc/playability_test_result/card_2026_03_19_13_25_04.md`
    - `doc/playability_test_result/card_2026_03_22_15_56_13.md`

## Fresh same-slice evidence
- fresh `software_safe` rerun run id: `20260409-172309`
- artifact root: `output/playwright/viewer-software-safe-step-retention/20260409-172309/`
- direct result:
  - `renderMode=software_safe`
  - `stepAccepted=true`
  - `selectedAgentVisible=true`
  - `domFeedbackVisible=true`
  - `logicalTimeAdvanced=false`
  - `eventSeqAdvanced=false`
  - `feedbackStage=blocked`
  - `failCategory=no_progress_after_step`
- blocker signature:
  - `http request failed: request timed out after 10000ms: error sending request for url (https://api.letai.run/v1/responses)`
- player-facing impact:
  - formal lane 仍停在 `first_session_loop.create_first_world_feedback`
  - `recentFeedback.effect` 明确写成 `runtime play loop stopped because the LLM decision provider failed`
  - `lastControlFeedback.effect` 明确写成 `gameplay blocked before requested advance completed`
- why the older PASS is insufficient:
  - `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 记录了 2026-04-08 的历史 PASS，但本专题要求的是当前 retention slice 的 fresh same-window gate truth。
  - 当前 worktree fresh rerun 仍然在 formal lane 第一步被 LLM provider timeout 阻断，因此不能继续沿用 2026-04-08 的 PASS 为本专题背书。

## Gate summary
- formal lane sample count:
  - qualified 10-minute samples: `0 / 3`
  - attempted fresh samples in current slice: `1`
  - abort reason: `software_safe` floor 先于 retention sampling 被 fresh blocker 打断
- `software_safe` floor verdict: `blocker`
- headed Web/UI retention verdict: `not_started`
- QA gate input: `hold`

## Producer verdict
- producer decision: `hold`
- rationale:
  - `PRD-GAME-012` 明确要求任一正式入口若仍存在最小控制 floor 失败，则本专题默认保持 `hold`。
  - 当前 blocker 发生在正式 active-LLM lane，而不是 debug/probe lane；因此不能再用 no-LLM 中循环正反馈冲淡 formal gameplay 失败。
  - 在 `software_safe` 第一步仍会因 provider timeout 卡死的情况下，继续追 3 个 retention 样本只会放大噪声，不会提高结论质量。

## Required follow-up before re-open
- 先修 active-LLM formal lane 的 provider timeout / request budget 问题，使 `software_safe` 第一步重新达到 `logicalTimeAdvanced=true` 与 `eventSeqAdvanced=true`。
- floor 恢复后，再补至少 3 个 active-LLM 10 分钟 retention 样本，并重新填写 `continue_playing / hold` gate。
- 继续保留 debug/probe lane 的 `--no-llm` 工业与 UI 语义回归，但这些样本不得再作为 formal retention 结论。
