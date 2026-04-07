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
- `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_gameplay_action_script_mode_requires_llm_mode -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_step_control_reports_blocked_without_llm_mode -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher parse_options_accepts_agent_direct_connect_alias -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher build_launcher_args_accepts_agent_direct_connect_alias -- --nocapture`

## 状态
- 更新日期: 2026-04-07
- 当前状态: completed
- 下一任务: 无
- 最新完成: `T1/T2/T3/T4/T5/T6/T7`（已完成三模式总契约建模、core 主入口挂载、下游术语回写、`pure_api` 的 LLM-required 正式游玩口径收口、`agent_direct_connect` / `openclaw_local_http` / execution lane 的多层术语收口，以及 `non-3D` / `software_safe` 的 priority-vs-mode 边界澄清）。
- 备注:
  - 本专题只冻结 taxonomy 与 claim contract，不替代下游专题实现。
  - 后续若新增同层玩家访问模式，必须先更新本专题再更新模块文档。
  - `--no-llm` 仍可作为观战/调试旁路保留，但不能再被写成正式可玩、parity 或发布放行入口。
  - `agent_provider_mode` CLI / config key 暂不改名；`agent_direct_connect` 只作为向前兼容 alias 暴露，内部 canonical provider implementation 仍保持 `openclaw_local_http`。
  - `non-3D` / `2D 优先` 只允许描述阶段优先级或交互范围；若要表达玩家入口，必须显式写回 `standard_3d / software_safe / pure_api`。
