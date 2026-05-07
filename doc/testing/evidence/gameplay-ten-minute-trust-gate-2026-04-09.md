# Gameplay trust gate verdict (2026-04-09 baseline, revalidated 2026-04-15)

审计轮次: 2

## Meta
- 关联专题: `PRD-GAME-012`
- 关联任务: `TASK-GAME-065`
- 责任角色: `qa_engineer`
- 裁决角色: `producer_system_designer`
- 当前结论: `hold`
- 目标: 在当前 10 分钟留存修复切片下，把 active-LLM formal lane 的 `trust gate` 与后续 `first capability gate` 分开裁决；只有在 `software_safe` floor 通过后，才允许继续累计更长正式样本。

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

## Trust-gate revalidation (2026-04-15)
- revalidation task:
  - `.pm/tasks/task_1dbcc087ae374721aa0928de3cd240e2.yaml`
- config source:
  - 当前 worktree 复用了源仓 `config.toml` 的 real-provider 配置副本；该文件仅用于本轮 local real-provider 复跑，不纳入提交。
- shared active-LLM stack:
  - startup run id: `20260415-223459`
  - game url: `http://127.0.0.1:4673/?ws=ws://127.0.0.1:5511&test_api=1`
  - stack logs: `output/playwright/playability/startup-20260415-223459/`
  - ready proof:
    - chain runtime `ok=true`
    - viewer/game stack emitted ready URL and stayed alive for follow-up probes
- floor probe A:
  - run id: `20260415-224143`
  - artifact root: `output/playwright/viewer-software-safe-step/20260415-224143/`
  - direct facts:
    - `initial_state.connectionStatus=connected`
    - `initial_state.renderMode=software_safe`
    - `initial_state.goalId=first_session_loop.create_first_world_feedback`
    - `step_request.accepted=true`
    - `step_request.stage=queued`
    - `step_request.deltaLogicalTime=0`
    - `step_request.deltaEventSeq=0`
    - 脚本在默认 `15000ms` 窗口内未拿到 terminal feedback，也未生成 `after_step_state.json` / `final_state.json`
- floor probe B:
  - run id: `20260415-224312`
  - artifact root: `output/playwright/viewer-software-safe-step/20260415-224312/`
  - direct facts:
    - 与 probe A 相同的 shared URL / same-stack 条件下再次复现
    - `step_request.accepted=true`
    - `step_request.stage=queued`
    - `initial_state.logicalTime=1`
    - `initial_state.eventSeq=0`
    - 即使把 step wait window 放宽到 `30000ms`，依然没有 terminal feedback，也没有任何世界时间推进证据
- revalidation verdict:
  - 当前 rerun 不是 `Responses API` 10 秒 timeout 旧签名，也不是浏览器/stack 冷启动失败。
  - 当前 blocker 已回退到更早的 trust floor：active-LLM formal lane 的首个 `step` 可以被接受，但会长期停留在 `queued`，且不给出 `lastControlFeedback` 终态，也不推进 `logicalTime/eventSeq`。
  - 按当前 gate 规则，本轮不继续跑 `./scripts/collect-active-llm-retention-sample.sh`，因此 `first capability gate` 结论保持 `not_run`。

## Formal 10-minute evidence

### Qualified 10-minute trust samples

| 样本 | run id | 工件目录 | 结果卡 | 关键事实 | QA 结论 |
| --- | --- | --- | --- | --- | --- |
| A | `20260410-125829` | `output/playwright/retention-active-llm-formal/active-llm-retention-20260410-125829/` | `doc/playability_test_result/card_2026_04_10_12_58_29.md` | `playDurationMs=600000`，`reachedPostOnboarding=true`，`maxLogicalTime=43`，`finalGoalId=post_onboarding.establish_first_capability`，`finalProgressPercent=20` | formal lane 稳定连通，说明 trust path 至少不是“第一步就停机”；但该样本只足以说明 trust 有希望，不足以单独证明 `first capability gate`。 |
| B | `20260410-132858` | `output/playwright/retention-active-llm-formal/active-llm-retention-20260410-132858/` | `doc/playability_test_result/card_2026_04_10_13_28_58.md` | 先到 `post_onboarding / 20%`，随后在样本中后段 UI 语义回退到 `first_session_loop.create_first_world_feedback / 0%`，且 `logicalTime=22`、`eventSeq=7` 在余下样本保持不变 | formal lane 不只是 capability 没闭环，还出现 trust 级别的阶段回退并伴随时间冻结，属于更强的 blocker。 |
| C | `20260410-134323` | `output/playwright/retention-active-llm-formal/active-llm-retention-20260410-134323/` | `doc/playability_test_result/card_2026_04_10_13_43_23.md` | 先到 `post_onboarding / 20%`，随后在样本前中段即回退到 `first_session_loop.create_first_world_feedback / 0%`，且 `logicalTime=13`、`eventSeq=6` 在余下样本保持不变 | B 的回退冻结签名在另一独立 10 分钟样本中再次复现，说明当前 `10-minute trust gate = hold` 不是单次偶发噪音。 |

### Player leverage rubric overlay (`#166`)
- 本轮额外按 `player leverage` 口径复核 5 个问题：
  - 玩家做了什么关键动作？
  - 世界有什么变化可以归因到这个动作？
  - 玩家能否解释结果为什么发生？
  - 结果有没有打开新的决策？
  - 这条链路会不会促使玩家继续玩？
- 评分规则：每项 `0/1`，合计 `player_leverage_score=0~5`；`4~5 => pass`，`2~3 => watch`，`0~1 => block`。

| 样本 | 玩家关键动作 `player_action` | 世界因此变化 `world_change_due_to_player` | `player_leverage_score` | `leverage verdict` | `world_activity_only` | 说明 |
| --- | --- | --- | --- | --- | --- | --- |
| A | formal lane 下持续推进 controls，成功把会话带到 `PostOnboarding` | 世界显式进入 `post_onboarding.establish_first_capability`，并给出 `goal/progress` | `3/5` | `watch` | `no` | 这条样本证明玩家至少能把世界推进到下一阶段，但当前包里仍不足以证明“哪一个玩家选择”打开了一个会让人想继续玩的新决策，因此不能只凭世界活跃或阶段推进就判成 leverage pass。 |
| B | 玩家同样把样本推进到 `PostOnboarding` | 世界先前进，随后语义回退到 `first_session_loop` 并冻结 | `1/5` | `block` | `no` | 该样本不能稳定证明玩家持续拥有杠杆；更强事实是“玩家推进后系统回退并冻结”，所以它是 trust blocker，而不是 world activity 成功样本。 |
| C | 玩家推进 formal lane 并短暂进入 `PostOnboarding` | 世界很快回退并冻结，新的决策没有稳定打开 | `1/5` | `block` | `no` | 与 B 同签名复现，说明当前失败不是“世界不够热闹”，而是玩家刚获得一点杠杆就丢失了它。 |

### Player leverage verdict
- 当前正式样本不是“只有 world activity、完全没有 player leverage”。
- 但当前也不能给出稳定 `player leverage pass`：
  - A 只有 `watch`
  - B/C 直接 `block`
- 因此当前 trust gate 的 `hold` 必须继续保持；即使世界曾经活跃、阶段曾经推进，也不能把它误判成“玩家已经获得稳定而有意义的参与感”。

### Supplemental shorter samples
- `20260410-111006`
  - `playDurationMs=300000`
  - 已到 `post_onboarding.establish_first_capability`
  - `finalProgressPercent=20`
  - 说明 5 分钟窗口内也只看到“到达 20%”而不是“完成首个可持续能力”；该事实归入 capability gate，而不是单独决定 trust gate
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
  - qualified 10-minute trust samples: `3 / 3`
  - supplemental 300s samples: `3`
  - excluded samples: `2`
- `software_safe` floor verdict:
  - historical baseline (2026-04-09 / 2026-04-10): `pass`
  - current rerun (2026-04-15): `hold`
- active-LLM trust gate verdict: `hold`
- first capability gate verdict: `not_run`
- QA gate input: `hold`
- exact blocker signature:
  - 2026-04-15 current blocker：在 shared active-LLM stack 上，`step_request.accepted=true` 但持续卡在 `stage=queued`，`deltaLogicalTime=0`、`deltaEventSeq=0`，且在 `15000ms` 与 `30000ms` 两档窗口内都拿不到 terminal feedback
  - 样本 A 证明当前阻断不再是 provider timeout 或首步 floor 崩溃，而是 formal lane 在 `post_onboarding.establish_first_capability` 长时间卡在 `20%`
  - 样本 B/C 进一步证明存在更强阻断：同一正式 lane 会在进入 `post_onboarding / 20%` 后回退到 `first_session_loop.create_first_world_feedback / 0%`，且 `logicalTime/eventSeq` 冻结不再增长

## Producer verdict
- producer decision: `hold`
- rationale:
  - 历史 10 分钟样本仍然证明 formal lane 的 progression/retention 尚未闭环。
  - 但 2026-04-15 这轮复验甚至没有走到累计 10 分钟正式样本的前提，因为 trust floor 已重新卡在 `step accepted -> queued forever`。
  - 在 trust gate 未恢复前，继续讨论 capability gate 或 `continue_playing` 都会污染正式结论。

## Required follow-up before re-open
- `runtime_engineer` 已在 `task_8d2e20dd7f5c47fd8303ff55159227ba` 清除一条 2026-05-07 当前 `main` 的更前置 startup blocker：fresh `run-game-test --json-ready` 不再因 `reward-runtime-execution-world` 缺少初始 `snapshot.json/journal.json` 而在 Viewer HTTP ready 前退出。下一轮 trust-gate 复验应基于已恢复的 launcher ready contract，而不是复用本文件中的 2026-04-15 bootstrap failure 前提。
- `producer_system_designer` 需要把当前对外口径更新为“active-LLM trust gate 仍 hold；capability gate 未进入”，停止任何 `continue_playing` 或“已恢复到 capability 检查”的延伸表述。
- `runtime_engineer` / `viewer_engineer` 需要先解释并修复当前最前置 blocker，再重新申请正式 trust 复验：
  - shared active-LLM stack 下，`step_request.accepted=true` 但长期停留 `queued`，不给 terminal `lastControlFeedback`
  - 同一时间窗口内 `logicalTime/eventSeq` 不前进，说明 first-step world delta 没有完成闭环
- 只有在 trust floor 再次恢复后，才允许重新检查历史 progression blocker：
  - `post_onboarding.establish_first_capability` 长时间停在 `20%`
  - `post_onboarding -> first_session_loop` 的阶段语义回退伴随 `logicalTime/eventSeq` 冻结
- 继续保留 debug/probe lane 的 `--no-llm` 工业与 UI 语义回归，但这些样本不得再作为 formal retention 结论。
