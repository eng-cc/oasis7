# oasis7：玩家访问模式总契约（2026-03-19）项目管理文档

- 对应设计文档: `doc/core/player-access-mode-contract-2026-03-19.design.md`
- 对应需求文档: `doc/core/player-access-mode-contract-2026-03-19.prd.md`

审计轮次: 7

## 任务拆解
- [x] T1 (`PRD-CORE-009`) [test_tier_required]: 冻结 `standard_3d / software_safe / pure_api` 三模式总契约，明确 mode/lane 分层、claim envelope、fallback 规则与禁宣称项。
- [x] T2 (`PRD-CORE-009`) [test_tier_required]: 同步 `doc/core/prd.md`、`doc/core/project.md`、`doc/core/prd.index.md` 与 `doc/core/README.md`，把本专题挂入 core 主入口。
- [x] T3 (`PRD-CORE-009`) [test_tier_required]: 回写 `doc/devlog/2026-03-19.md`，记录 owner、完成内容、验证方式与后续使用约束。
- [x] T4 (`PRD-CORE-009`) [test_tier_required]: 对齐 `testing-manual`、`doc/world-simulator/**`、`doc/game/**` 与 `doc/testing/**` 的下游术语，要求结论先绑定玩家访问模式，再附加 execution lane。
- [x] T5 (`PRD-CORE-009`) [test_tier_required]: 将 `pure_api` 的正式游玩口径重定为“必须启用且可连通 LLM”，同步回写 launcher/runtime 行为、README/testing/manual/game/world-simulator 当前入口，并把 `--no-llm` 降级为 observer/debug only。
- [x] T6 (`PRD-CORE-009`) [test_tier_required]: 将旧“OpenClaw mode”歧义口径收口为“`agent_direct_connect` 接入方式 + `openclaw_local_http` provider implementation + execution lane”，同步回写 core/world-simulator/testing 文档、launcher/client launcher 用户文案与兼容 alias。
- [x] T7 (`PRD-CORE-009`) [test_tier_required]: 收口 `non-3D` / `2D 优先` 与 `software_safe` 的边界，把阶段优先级话术明确降回 delivery priority / interaction scope，并同步回写 core 契约与 `world-simulator` 的 3D hold 主文档。
- [x] T8 (`PRD-CORE-009`) [test_tier_required]: 将 agent provider 正式配置收口为 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile + agent_execution_lane`，把 `agent_direct_connect/openclaw_local_http` 降为兼容 alias，并同步回写 core/world-simulator/testing 文档与 launcher/runtime 透传口径。
- [x] T9 (`PRD-CORE-009`) [test_tier_required]: 按最新产品设定重写三模式 claim envelope，把 `software_safe` 升格为主要正式 Web 入口、把 `standard_3d` 收口为 opt-in visual QA 模式、并保留 `pure_api` 的一等公民 no-UI 定位；同步回写本专题 PRD / design / project 与模块级 project 状态。
- [ ] T10 (`PRD-CORE-009`) [test_tier_required]: 在下游实现真正对齐后，再同步更新 `README.md`、`testing-manual.md` 与其他“当前入口/当前预览”载体，避免在实现落地前提前宣称主入口已切换。

## 依赖
- `doc/core/prd.md`
- `doc/core/project.md`
- `doc/core/prd.index.md`
- `doc/core/README.md`
- `testing-manual.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
- `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.prd.md`
- `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.prd.md`

## 验证
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- `rg -n "non-3D|玩家访问模式|delivery priority|interaction scope" doc/core/player-access-mode-contract-2026-03-19.{prd,design,project}.md doc/world-simulator/prd.md doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md`
- `rg -n "主要正式 Web 入口|formal Web gameplay|visual QA|一等公民" doc/core/player-access-mode-contract-2026-03-19.{prd,design,project}.md doc/core/{prd,project}.md doc/world-simulator/{prd,project}.md doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.{prd,design,project}.md`
- `rg -n "main_token_transfer|handoff|专门动作|not_exposed" doc/core/player-access-mode-contract-2026-03-19.{prd,design,project}.md doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.{prd,design,project}.md`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_gameplay_action_script_mode_requires_llm_mode -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_step_control_reports_blocked_without_llm_mode -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher parse_options_accepts_agent_direct_connect_alias -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher build_launcher_args_accepts_agent_direct_connect_alias -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher openclaw_viewer_live_env_sets_provider_specific_overrides_without_builtin_llm_timeout -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 openclaw_settings_from_env_parses_profile_and_timeout -- --nocapture`

## 状态
- 更新日期: 2026-04-07
- 当前状态: active
- 下一任务: T10
- 最新完成: `T1/T2/T3/T4/T5/T6/T7`（已完成三模式总契约建模、core 主入口挂载、下游术语回写、`pure_api` 的 LLM-required 正式游玩口径收口、`agent_direct_connect` / `openclaw_local_http` / execution lane 的多层术语收口，以及 `non-3D` / `software_safe` 的 priority-vs-mode 边界澄清）。
- 最新完成: `T8`（已将 agent provider 正式配置收口为 `agent_decision_source + agent_provider_* + agent_execution_lane` 结构化 taxonomy，并把 `agent_direct_connect/openclaw_local_http` 降为兼容 alias。）
- 最新完成: `T9`（已将 `software_safe` 重写为主要正式 Web 入口、将 `standard_3d` 收口为 visual QA 模式，并保留 `pure_api` 的一等公民 no-UI 角色。）
- 备注:
  - 本专题只冻结 taxonomy 与 claim contract，不替代下游专题实现。
  - 后续若新增同层玩家访问模式，必须先更新本专题再更新模块文档。
  - `--no-llm` 仍可作为观战/调试旁路保留，但不能再被写成正式可玩、parity 或发布放行入口。
  - 正式 operator-facing 配置、CLI 与 env 口径以 `agent_decision_source + agent_provider_backend/contract/transport/url/auth/connect_timeout_ms/profile + agent_execution_lane` 为准；`agent_provider_mode`、`agent_direct_connect` 与 `openclaw_local_http` 只允许作为兼容解析保留。
  - `non-3D` / `2D 优先` 只允许描述阶段优先级或交互范围；若要表达玩家入口，必须显式写回 `standard_3d / software_safe / pure_api`。
  - 在 T10 完成前，README / testing-manual 一类“当前预览入口”载体仍可保留现状描述，但不得与本专题新目标混写成“已实现”。
