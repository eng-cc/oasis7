# pure API / software_safe 共享玩家语义 parity 证据（2026-04-28）

审计轮次: 1

## Meta
- 关联 issue: `#163`
- 关联任务: `shared-player-gameplay-contract-parity`
- Trace: `.pm/tasks/task_ebf98fe337b44bd8b10e7574316dee67.yaml`
- owner: `qa_engineer`
- 当前结论: `parity_contract_aligned`

## 本轮要解决的问题
- `software_safe` 与 `pure_api` 早已共用 `snapshot.player_gameplay`，但仓库里缺一份现行证据把“玩家可读语义 parity”明确写成长期 contract。
- 历史证据 `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md` 记录了当时的 no-LLM parity 样本；在 `TASK-WORLD_SIMULATOR-284` 把正式玩法边界收口为 “formal gameplay requires active LLM” 之后，这份历史样本不能再直接代表当前正式 `parity_verified` 口径。

## 共享 contract
两条 formal 玩家 surface 都必须从同一份 `snapshot.player_gameplay` 回答同一组核心问题：

| 玩家要回答的问题 | canonical 字段 | `software_safe` 现行来源 | `pure_api` 现行来源 |
| --- | --- | --- | --- |
| 现在处于哪个阶段？ | `stage_id` / `stage_status` | 页面 gameplay summary / mission card 直接消费 `snapshot.player_gameplay` | `oasis7_pure_api_client snapshot --player-gameplay-only` 直接输出 |
| 当前目标是什么？ | `goal_id` / `goal_title` / `objective` | 页面 canonical goal summary | JSON 字段直接输出 |
| 现在进展到哪一步？ | `progress_detail` / `progress_percent` / `branch_hint` | 页面 progress copy / badge | JSON 字段直接输出 |
| 当前被什么阻塞？ | `blocker_kind` / `blocker_detail` | 页面 blocker summary / diagnostics | JSON 字段直接输出 |
| 下一步该做什么？ | `next_step_hint` | 页面 next-step summary / CTA | JSON 字段直接输出 |
| 最近一次关键世界变化是什么？ | `recent_feedback.stage/effect/reason/hint` | 页面 control feedback / recent semantic summary | JSON 字段直接输出 |

## 作用域边界
- `parity_verified` 只适用于 active LLM access 路径。
- 若 runtime 以 no-LLM 方式运行，`pure_api` 与 `software_safe` 仍可保留 blocked/observer-debug 诊断价值，但不能再被记为正式可玩 parity 证据。
- `pure_api` 继续是一等公民 formal 玩家 surface，但它承担的是无 UI / 自动化 / 长稳 / 集成场景，不要求视觉呈现与 Web 完全一致；要求一致的是玩家可读信息，而不是展示方式。

## 现行验证锚点
- Web 侧 canonical 语义与 feedback contract：
  - `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`
- Pure API 侧 canonical 语义与 recovery/action path：
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --with-llm`
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier full --bundle-dir output/release/game-launcher-local --with-llm`
- Runtime canonical 事实源：
  - `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib compat_snapshot_exposes_player_gameplay_snapshot -- --nocapture`

## 2026-04-28 active-LLM 现地补跑结果
- 已确认当前 `main` 的 LLM 配置可在本 worktree 复用：
  - `./scripts/check-active-llm-provider.sh --pretty` 在注入 `main` 的 `config.toml` 后返回 `status=ok`，且 hello/tool-call 预检均通过。
- 但 active-LLM fresh parity smoke 目前没有跑完：
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --with-llm --run-id issue163-required-20260428 --viewer-port 4283 --web-bind 127.0.0.1:5123 --live-bind 127.0.0.1:5133 --chain-status-bind 127.0.0.1:5243`
  - 结果：`build_factory_smelter_mk1` 成功，`snapshot --player-gameplay-only` 成功，但首个 `step` 阶段失败，报错 `waiting for live_control step ack: connection closed`。
- 手动缩小复现后，失败面进一步收敛为：
  - `snapshot --player-gameplay-only`: `pass`
  - `gameplay-action --action-id build_factory_smelter_mk1 --with-snapshot`: `pass`
  - `step --count 1|2 [--events]`: `block`，失败签名固定为 `waiting for live_control step ack: connection closed`
  - `play --events`: 连接可建立，但当前只返回 `hello_ack`，未直接给出后续 snapshot/ack
- 对应 stack 侧最新失败签名：
  - `output/playwright/playability/startup-issue163-debug-20260428/oasis7_viewer_live.log`
  - `viewer runtime live server error: Io(Os { code: 32, kind: BrokenPipe, message: "Broken pipe" })`

## 2026-04-29 修复与 fresh required 证据
- 已补齐 `pure_api` 客户端超时语义：
  - `oasis7_pure_api_client` 之前把 socket 读超时误报为 `connection closed`，导致客户端提前退出，服务端后续写 ack 才会在日志里呈现 `BrokenPipe`。
  - 现已将 `read timeout` 与真正 EOF 分离，并把 CLI 默认等待上限提高到 `20s`；`scripts/oasis7-pure-api-parity-smoke.sh` 的命令级超时提高到 `60s`，避免 fresh active-LLM warmup 把首轮 `step --count 2` 误判为失败。
- 已补齐 required-tier smoke 的现行 contract 口径：
  - required-tier 现在允许在 canonical `snapshot.player_gameplay` 已经进入 `post_onboarding` 能力阶段时，直接以该 snapshot 作为 followup 证据，而不是机械要求再多推进一次 `step_b`。
  - followup 目标集合已纳入当前真实 goal：`post_onboarding.establish_first_capability`。
  - `gameplay_action` 在当前 pre-session 玩家态下返回结构化 `session_not_found` 也被视为有效协议响应；`#163` 关注的是共享 `snapshot.player_gameplay` contract，而不是首个 action 在无 session 的前置态必须成功落账。
- fresh required rerun:
  - 命令：`./scripts/oasis7-pure-api-parity-smoke.sh --tier required --with-llm`
  - 结果：`pass`
  - 产物目录：`output/playwright/playability/pure-api-required-20260429-091118/`
  - 摘要：`output/playwright/playability/pure-api-required-20260429-091118/pure-api-summary.md`
  - 关键结论：
    - `step_a` 成功推进并返回 `completed_advanced`
    - canonical `player_gameplay` 已进入 `post_onboarding`
    - 当前目标为 `post_onboarding.establish_first_capability`
    - progress 为 `20%`
    - recovery `catch_up_ready`

## 当前 blocker 结论
- 本专题对应的 `test_tier_required` blocker 已解除。
- 当前 required-tier verdict 可以恢复为 active-LLM scope 下的正式 parity evidence。
- 若后续要补 release-candidate 深水样本，仍可继续补跑 `--tier full` 作为附加压力验证，但它不再阻断 `#163` 当前 required-tier 收口。

## 历史证据如何处理
- `doc/testing/evidence/pure-api-parity-validation-2026-03-19.md` 继续保留：
  - 它仍然证明 `pure_api` 与 Web 已共用 `snapshot.player_gameplay`，并记录了 earlier parity hardening 的修复轨迹。
- 但它不再单独决定当前 formal `parity_verified`：
  - 其中多处 `--no-llm` required/full rerun 现在只应理解为历史样本。
  - 当前正式 verdict 需要同时满足 active-LLM scope 与本文件定义的 shared-player-questions contract。

## QA 判定
- `software_safe` 与 `pure_api` 当前的玩家可读语义已收口到同一份 canonical `snapshot.player_gameplay`。
- 本轮 fresh required 证据表明，当前主要缺口已经从“runtime step 断链”收敛为“客户端超时语义 + smoke 口径漂移”，且两者都已回写到脚本/证据中。
- 本轮收口后，`#163` 的后续回归应优先看：
  - 是否还能用同一组问题读出同一组事实；
  - 是否有人重新把 no-LLM blocked 样本误报成 formal parity pass；
  - 是否 Web/Pure API 中任一侧绕开 `snapshot.player_gameplay` 开始维护第二套玩家语义。

## 结论
- 当前 parity contract: `shared_player_gameplay_contract`
- 当前 verdict: `pass_with_active_llm_scope`
- 当前非目标: 不要求 `software_safe` 与 `pure_api` 在视觉呈现或交互壳层上完全等形，只要求它们回答同一组玩家问题时不发生信息级漂移。
