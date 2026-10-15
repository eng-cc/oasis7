# 纯 API parity 验证证据（2026-03-19）

历史记录边界：以下判定、命令、角色交接及“当前/下一步”均属于文中明确的采集窗口；保留原始失败、通过、未执行范围和审计定位，不作为当前规范、进度或持续跟踪义务。当前行动与验收回到[测试入口](../README.md)及对应专业 authority，使用新的候选和证据窗口；后续通过、合入或任务关闭不改写本次运行事实。

本次 `--no-llm` parity 结论仅用于 2026-03 样本。后续 active-LLM 口径演进见[2026-04 共享玩家语义记录](pure-api-shared-player-gameplay-parity-2026-04-28.md)，当前正式范围仍以测试与产品 authority 为准。

审计轮次: 2

## Meta
- 关联专题: `PRD-GAME-008 / 纯 API 客户端等价玩法`
- 关联任务: `TASK-GAMEPLAY-API-004`
- 责任角色: `qa_engineer`
- 当前结论: `parity_verified`
- 目标: 用可复跑的 required/full 证据与 parity matrix 区分“纯 API 已可持续基础游玩”与“纯 API 已达到 UI 等价”。

## 最终收口（2026-03-19 15:18 CST）
- `runtime_engineer` 已定位并修复 pure API fresh no-LLM 路径中的真实阻断：
  - 根因不是 `job_id` 字段值错误，而是 `runtime_snapshot` 内部带 `u64` 键的 `BTreeMap`（例如 `pending_factory_builds`）在 JSON 中被编码为字符串键，native 反序列化会在首次工厂建造 `step` 后报 `invalid type: string "6", expected u64`。
  - 修复方案是在 runtime `Snapshot / WorldState` 的所有数值键 map 上统一接入字符串键兼容反序列化，并补充 `snapshot_runtime_snapshot_accepts_stringified_numeric_map_keys` 回归。
- 修复后追加 source required/full 复验：
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --no-llm --viewer-port 4297 --web-bind 127.0.0.1:5161 --live-bind 127.0.0.1:5163 --chain-status-bind 127.0.0.1:5249`
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier full --no-llm --viewer-port 4299 --web-bind 127.0.0.1:5171 --live-bind 127.0.0.1:5173 --chain-status-bind 127.0.0.1:5251`
- 复验结果：
  - required/full 都已通过。
  - pure API 现已稳定完成 `build_factory_smelter_mk1 -> schedule_recipe_smelter_iron_ingot -> PostOnboarding -> choose_midloop_path`。
  - `reconnect-sync --with-snapshot` 继续可恢复 `latest_snapshot + player_gameplay`。

## 后续修复跟进（2026-03-19）
- `runtime_engineer` 已修复 `oasis7_pure_api_client reconnect-sync --with-snapshot`，现在会显式补拉 `snapshot`，恢复结果可直接携带 `latest_snapshot + player_gameplay`。
- `viewer_engineer` 已把 Mission HUD / PostOnboarding 主卡改为优先消费 canonical `snapshot.player_gameplay`；只有缺少协议级玩家语义时才回退旧事件聚合逻辑。
- 修复后追加 source required 回归：
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --no-llm --viewer-port 4283 --web-bind 127.0.0.1:5123 --live-bind 127.0.0.1:5133 --chain-status-bind 127.0.0.1:5243`
  - 产物：`output/playwright/playability/pure-api-required-20260319-135315/`
  - 新增检查 `recovery_snapshot_present / recovery_player_gameplay_present` 均已通过。
- 定向验证：
  - `env -u RUSTC_WRAPPER cargo test -q -p oasis7 --bin oasis7_pure_api_client`
  - `env -u RUSTC_WRAPPER cargo test -q -p oasis7_viewer build_player_post_onboarding_snapshot_prefers_canonical_player_gameplay_snapshot`
  - `env -u RUSTC_WRAPPER cargo test -q -p oasis7_viewer build_player_post_onboarding_snapshot_uses_canonical_blocker_fields`
  - `env -u RUSTC_WRAPPER cargo check -q -p oasis7 -p oasis7_viewer`
  - `env -u RUSTC_WRAPPER cargo build -q -p oasis7 --bin oasis7_pure_api_client`
  - `target/debug/oasis7_pure_api_client --addr 127.0.0.1:5132 reconnect-sync --player-id player-api-smoke --with-snapshot`

## 同候选 fresh bundle 复验（2026-03-22 18:38 CST）
- 为 closed-beta unified gate 复跑同候选 no-LLM bundle required/full：
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --no-llm`
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier full --bundle-dir output/release/game-launcher-local --no-llm`
- fresh rerun 结果：
  - required/full 均 `pass`。
  - 两条链路都到达 `post_onboarding.choose_midloop_path`，且 `followup_progress_percent=100`。
  - required/full 继续保持 `reconnect-sync` 恢复语义；full 继续通过 `step_c` 抽样。
- 该轮的作用不是改变专题结论，而是证明 `parity_verified` 已在当前 `closed_beta_candidate` 候选包上持续成立，可正式并入 unified gate。

## 执行命令
- source required-tier 预跑:
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --no-llm --viewer-port 4277 --web-bind 127.0.0.1:5117 --live-bind 127.0.0.1:5127 --chain-status-bind 127.0.0.1:5237`
- fresh bundle required-tier 正式口径:
  - `./scripts/build-game-launcher-bundle.sh --out-dir output/release/game-launcher-local`
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --no-llm --viewer-port 4278 --web-bind 127.0.0.1:5118 --live-bind 127.0.0.1:5128 --chain-status-bind 127.0.0.1:5238`
- bundle full-tier 抽样:
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier full --bundle-dir output/release/game-launcher-local --no-llm --viewer-port 4279 --web-bind 127.0.0.1:5119 --live-bind 127.0.0.1:5129 --chain-status-bind 127.0.0.1:5239`
- source required-tier 收口复验:
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --no-llm --viewer-port 4297 --web-bind 127.0.0.1:5161 --live-bind 127.0.0.1:5163 --chain-status-bind 127.0.0.1:5249`
- source full-tier 收口复验:
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier full --no-llm --viewer-port 4299 --web-bind 127.0.0.1:5171 --live-bind 127.0.0.1:5173 --chain-status-bind 127.0.0.1:5251`
- 同候选 bundle required-tier 复验:
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier required --bundle-dir output/release/game-launcher-local --no-llm`
- 同候选 bundle full-tier 复验:
  - `./scripts/oasis7-pure-api-parity-smoke.sh --tier full --bundle-dir output/release/game-launcher-local --no-llm`

## 产物路径
- source required-tier:
  - `output/playwright/playability/pure-api-required-20260319-131630/`
- bundle required-tier:
  - `output/playwright/playability/pure-api-required-20260319-132259/`
- bundle full-tier:
  - `output/playwright/playability/pure-api-full-20260319-132316/`
- source required-tier 收口复验:
  - `output/playwright/playability/pure-api-required-20260319-151759/`
- source full-tier 收口复验:
  - `output/playwright/playability/pure-api-full-20260319-151847/`
- 同候选 bundle required-tier 复验:
  - `output/playwright/playability/pure-api-required-20260322-183650/`
- 同候选 bundle full-tier 复验:
  - `output/playwright/playability/pure-api-full-20260322-183750/`
- 对应启动日志:
  - `output/playwright/playability/startup-20260319-131630/`
  - `output/playwright/playability/startup-20260319-132300/`
  - `output/playwright/playability/startup-20260319-132317/`
  - `output/playwright/playability/startup-20260319-151759/`
  - `output/playwright/playability/startup-20260319-151847/`
  - `output/playwright/playability/startup-20260322-183721/`
  - `output/playwright/playability/startup-20260322-183750/`

## required/full 结果摘要
- source required-tier:
  - `initial_stage=first_session_loop`
  - `followup_stage=post_onboarding`
  - `followup_goal_id=post_onboarding.establish_first_capability`
  - `reconnect_sync_ack=catch_up_ready`
- bundle required-tier:
  - 同样通过 `hello_live_profile / step_a_advanced / step_b_advanced / followup_stage_post_onboarding / reconnect_sync_ack`
  - `followup_time=33`
- bundle full-tier:
  - 额外 `step_c_advanced=true`
  - 长步进后仍能继续读取 `player_gameplay`
  - 但 `goal_id` 仍停留在 `post_onboarding.establish_first_capability`，未证明已到“首个持续能力里程碑”
- source required-tier 收口复验:
  - `initial_stage=first_session_loop`
  - `followup_stage=post_onboarding`
  - `followup_goal_id=post_onboarding.choose_midloop_path`
  - `followup_progress_percent=100`
  - `reconnect_sync_ack=catch_up_ready`
- source full-tier 收口复验:
  - `followup_goal_id=post_onboarding.choose_midloop_path`
  - `followup_progress_percent=100`
  - `step_c_advanced=true`
  - 继续步进后仍能稳定保留 `player_gameplay` 与最新反馈
- 同候选 bundle required-tier 复验:
  - `followup_goal_id=post_onboarding.choose_midloop_path`
  - `followup_progress_percent=100`
  - `reconnect_sync_ack=catch_up_ready`
- 同候选 bundle full-tier 复验:
  - `followup_goal_id=post_onboarding.choose_midloop_path`
  - `followup_progress_percent=100`
  - `step_c_advanced=true`
  - 继续步进后仍能稳定保留 `player_gameplay` 与最新反馈

## UI/API parity matrix
| 检查项 | Web/UI 当前状态 | Pure API 当前状态 | 结论 |
| --- | --- | --- | --- |
| 阶段 / 目标 / 进度 / 阻塞 / 下一步建议字段存在 | UI 的 Mission HUD / PostOnboarding 主卡已优先消费 `snapshot.player_gameplay` canonical 语义 | API 已由 `player_gameplay` canonical 字段直接提供 | `pass` |
| 基础推进动作（snapshot / step / play） | 可用 | `oasis7_pure_api_client` 与 `player_gameplay.available_actions` 已可用 | `pass` |
| 最近控制反馈 | UI 可见 | `recent_feedback + control_completion_ack` 已可读 | `pass` |
| 阶段承接（FirstSessionLoop -> PostOnboarding） | UI 已有 headed / no-UI 证据 | source 与 fresh bundle pure API 都已通过 | `pass` |
| 恢复面（重连后直接恢复阶段语义） | UI 重连后可再次看到任务卡 | `reconnect-sync --with-snapshot` 现已直接返回 recovery ack + snapshot + `player_gameplay` | `pass` |
| 正式玩家动作面（签名 `gameplay_action` / reconnect / snapshot） | UI 已有入口 | pure API required/full 已实证覆盖 build/schedule/reconnect 的正式动作链路；`agent_chat/prompt_control` 仍按 no-LLM 产品约束返回 `llm_mode_required`，不构成本轮阻断 | `pass` |
| 到达首个持续能力里程碑 | UI 工业链已有单独专题卡组与 evidence | pure API source required/full 收口复验已到 `post_onboarding.choose_midloop_path`，并达到 `progress_percent=100` | `pass` |
| UI/API 语义是否共用单一事实源 | UI 与 API 已共用 canonical `snapshot.player_gameplay` 作为 PostOnboarding 主语义源 | API 已改为 canonical `player_gameplay` | `pass` |

## QA 判定
- 当前 pure API 路径已经稳定超出 `observer_only / playable`。
  - 原因: 它现在能直接读取 canonical 玩家语义、列出正式玩法动作、推进 live 世界、恢复阶段语义，并在 fresh no-LLM required/full 路径到达首个持续能力里程碑。
- 当前 pure API 路径可记为 `parity_verified`。
  - 原因 1: required/full 最新证据已经证明 `FirstSessionLoop -> PostOnboarding -> choose_midloop_path` 持续闭环可达。
  - 原因 2: UI 与 API 共用 `snapshot.player_gameplay` canonical 事实源，信息粒度与阶段语义不再分叉。
  - 原因 3: `gameplay_action` 正式动作链路已覆盖 no-LLM required/full 路径的关键推进动作。

## 结论
- 当前 parity level: `parity_verified`
- 当前 parity verdict: `pass`
- release / 对外口径:
  - 可以说“纯 API 客户端已经具备与 UI 等价的 no-LLM 持续游玩闭环、canonical 玩家语义和正式推进动作面”
  - 仍应注明：视觉表现、headed Web 截图语义与 LLM 模式专属动作不是本专题 required/full 的判定范围

## 下一步建议
- 非阻断 follow-up:
  - `qa_engineer`: 继续把 `output/playwright/playability/pure-api-required-20260322-183650/` 与 `output/playwright/playability/pure-api-full-20260322-183750/` 作为当前 candidate 的 pure API gate 锚点，防止未来回退。
  - `agent_engineer`: 如后续要把 `agent_chat / prompt_control` 纳入 pure API parity，请单独定义 LLM 模式下的等价专题与验收口径。
