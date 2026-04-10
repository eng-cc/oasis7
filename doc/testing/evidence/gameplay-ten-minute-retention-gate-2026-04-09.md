# Gameplay 10-minute retention gate verdict (2026-04-09)

审计轮次: 1

## Meta
- 关联专题: `PRD-GAME-012`
- 关联任务: `TASK-GAME-065`
- 责任角色: `qa_engineer`
- 裁决角色: `producer_system_designer`
- 当前结论: `watch`
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
- failed fresh sample run id: `20260409-172309`
- failed artifact root: `output/playwright/viewer-software-safe-step-retention/20260409-172309/`
- failed direct result:
  - `renderMode=software_safe`
  - `stepAccepted=true`
  - `selectedAgentVisible=true`
  - `domFeedbackVisible=true`
  - `logicalTimeAdvanced=false`
  - `eventSeqAdvanced=false`
  - `feedbackStage=blocked`
  - `failCategory=no_progress_after_step`
- failed blocker signature:
  - `http request failed: request timed out after 10000ms: error sending request for url (https://api.letai.run/v1/responses)`
- failed player-facing impact:
  - formal lane 停在 `first_session_loop.create_first_world_feedback`
  - `recentFeedback.effect` 明确写成 `runtime play loop stopped because the LLM decision provider failed`
  - `lastControlFeedback.effect` 明确写成 `gameplay blocked before requested advance completed`
- real-main-config rerun:
  - run id: `20260409-225330`
  - artifact root: `output/playwright/viewer-software-safe-step-real-provider/20260409-225330/`
  - setup: 将 `main` worktree 根目录的 real-provider `config.toml` 复制到当前 task worktree 根目录，仅用于本轮 local real-provider 复跑，不纳入提交。
  - direct result:
    - `renderMode=software_safe`
    - `stepAccepted=true`
    - `selectedAgentVisible=true`
    - `domFeedbackVisible=true`
    - `logicalTimeAdvanced=true`
    - `eventSeqAdvanced=false`
    - `feedbackStage=completed_advanced`
    - `failCategory=None`
  - lane impact:
    - `final_state.json` 显示 `connectionStatus=connected`、`logicalTime=2`，`lastControlFeedback.effect=world advanced: logicalTime +1, eventSeq +0`。
    - formal lane 首个 `step` 已恢复 first-step progress，说明当前真实 provider + repo-root config 组合下，`software_safe` 最小控制 floor 已从 `hold` blocker 恢复。
- why the older PASS was previously insufficient, and why this rerun changes the gate input:
  - `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 记录了 2026-04-08 的历史 PASS，但本专题要求的是当前 retention slice 的 fresh same-window gate truth。
  - `20260409-172309` 证明同日 fresh slice 一度仍受 `10000ms` timeout 阻断，因此旧 PASS 不能直接为当前 retention gate 背书。
  - `20260409-225330` 在复制 main 的 real config 后恢复了 first-step progress，足以把当前专题从“floor blocker 导致 hold”推进到“floor 已恢复、继续 retention sampling”的 `watch` 状态。

## Gate summary
- formal lane sample count:
  - qualified 10-minute samples: `0 / 3`
  - attempted fresh samples in current slice: `2`
  - current note: `software_safe` floor 已恢复，但本轮仅完成最小控制 floor 复核，尚未开始新的 10 分钟 retention sample。
- `software_safe` floor verdict: `pass`
- headed Web/UI retention verdict: `watch`
- QA gate input: `watch`

## Producer verdict
- producer decision: `watch`
- rationale:
  - `PRD-GAME-012` 的 `AC-5` 只在正式入口仍存在最小控制 floor 失败时强制 `hold`；当前 real-main-config rerun 已恢复 `logicalTimeAdvanced=true`，该 blocker 已解除。
  - 当前证据只覆盖 formal lane 的 first-step progress 恢复，还没有 `3` 条 active-LLM 10 分钟 retention 样本，因此仍不能直接宣称 `continue_playing`。
  - producer 当前可接受的事实口径是：正式入口已恢复到可继续采样的 `watch` 状态，下一步应立即补 retention samples，而不是继续停在 floor blocker 排障。

## Required follow-up before re-open
- 继续使用当前 real-provider + main config 环境补至少 `3` 条 active-LLM 10 分钟 retention 样本，并重新填写 `continue_playing / hold` gate。
- 若后续样本再次退回 `10000ms` timeout 或其他 formal lane blocker，需按 run id 精确记录失败签名，避免把单次恢复误写成长期稳定性结论。
- 继续保留 debug/probe lane 的 `--no-llm` 工业与 UI 语义回归，但这些样本不得再作为 formal retention 结论。
