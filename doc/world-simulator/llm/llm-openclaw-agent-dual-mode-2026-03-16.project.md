# Agent 直连执行 Lane（OpenClaw provider: player_parity / headless_agent / debug_viewer）（2026-03-16）项目管理文档

- 对应需求文档: `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.prd.md`
- 关联专题:
  - `doc/world-simulator/llm/llm-openclaw-agent-experience-parity-2026-03-12.project.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 完成 agent 直连 execution lane 专题 PRD / Project 建模，并回写模块主文档、索引与 devlog。
- [x] T1 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `agent_engineer` 牵头冻结 `player_parity` / `headless_agent` 的 observation/action contract、schema version 与禁止泄露的真值边界，并形成 supporting spec `openclaw-agent-dual-mode-contract-2026-03-16.md`。
- [x] T2 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `runtime_engineer` 落地 mode metadata、统一 replay/summary 追踪字段，并确保所有模式共享权威动作校验。
- [x] T3 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `viewer_engineer` 把 `debug_viewer` 明确收口为旁路订阅层，并补 mode/fallback 可观测性与 software-safe 对照入口。
- [x] T3.5 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `agent_engineer` 接通真实 `player_parity` 执行 lane 到 runtime live / `oasis7_game_launcher` / `oasis7_openclaw_parity_bench` / `oasis7`，并完成 `headless_agent` / `player_parity` 双 smoke 采证，解除 T4 代码阻断。
- [x] T3.6 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `producer_system_designer` 重构 `oasis7` operator 入口说明，把 `headless_agent`、`player_parity`、`debug_viewer` 与 `software_safe` 的职责分层直接写到技能文档，避免只读 skill 的操作者把 Viewer 误解为 OpenClaw 主执行依赖。
- [x] T3.7 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `producer_system_designer` 将 `oasis7` 中非关键的 UI/observer 说明拆到独立 reference，保持主 skill 优先暴露 `headless_agent` / `player_parity` 的执行路径。
- [x] T4 (PRD-WORLD_SIMULATOR-040) [test_tier_full]: 由 `qa_engineer` / `producer_system_designer` 对同一 OpenClaw 场景执行 `player_parity` vs `headless_agent` 对照采证，形成默认模式与阻断结论。
- [x] T4.1 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `producer_system_designer` 固化 2026-04-06 formal review，记录当前 `agent_direct_connect` 在 launcher 可达性、dual-mode observation、provider handshake 与 fallback 审计链上的 confirmed gap，并回写 follow-up owner/顺序。
- [x] T4.2 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `viewer_engineer` 落地 client launcher 的 `OpenClaw execution mode` 配置与透传，并统一 launcher / operator 的 timeout policy，消除 GUI 主链路默认静默落回 `headless_agent` 与 `200ms` 假超时。
- [x] T4.3 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `agent_engineer` / `runtime_engineer` 落地真实 `player_parity` / `headless_agent` observation 分层、schema mismatch 结构化失败与 fixture diff 验证，确保 dual-mode 不再只是 metadata 标签。
- [x] T4.4 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `viewer_engineer` / `runtime_engineer` 为 launcher provider compatibility check、runtime live snapshot、viewer debug context 与 parity summary 补 `capabilities/supported_action_sets` 兼容校验与 `fallback_reason` 透传，收口 `ready/degraded/incompatible` 判定与 `agent_direct_connect` alias fallback 审计链。
- [x] T4.5 (PRD-WORLD_SIMULATOR-040) [test_tier_full]: 由 `qa_engineer` / `producer_system_designer` 在 T4.2~T4.4 完成后重跑 dual-mode 真实采证，并重新判断本专题是否可恢复 `completed`。产物文件: `doc/world-simulator/llm/openclaw-agent-dual-mode-recertification-2026-04-07.md`、`doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.project.md`、`doc/world-simulator/project.md`。验收命令 (`test_tier_full`): `openclaw --version`；`curl -sS http://127.0.0.1:18789/health`；`curl -sS http://127.0.0.1:5841/v1/provider/health | jq .`；`curl -sS http://127.0.0.1:5841/v1/provider/info | jq .`；`env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/tmp/oasis7-task298-target bash scripts/openclaw-parity-p0.sh --openclaw-only --samples 1 --ticks 4 --timeout-ms 15000 --openclaw-base-url http://127.0.0.1:5841 --openclaw-connect-timeout-ms 15000 --openclaw-agent-profile oasis7_p0_low_freq_npc --execution-mode headless_agent`；`env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/tmp/oasis7-task298-target bash scripts/openclaw-parity-p0.sh --openclaw-only --samples 1 --ticks 4 --timeout-ms 15000 --openclaw-base-url http://127.0.0.1:5841 --openclaw-connect-timeout-ms 15000 --openclaw-agent-profile oasis7_p0_low_freq_npc --execution-mode player_parity`

## 依赖
- `doc/world-simulator/llm/llm-openclaw-agent-experience-parity-2026-03-12.prd.md`
- `doc/world-simulator/llm/llm-openclaw-agent-experience-parity-2026-03-12.project.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md`
- `doc/world-simulator/llm/llm-openclaw-local-http-provider-integration-2026-03-12.prd.md`
- `doc/world-simulator/llm/llm-decision-provider-standard-openclaw-feasibility-2026-03-12.prd.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `doc/world-simulator/llm/openclaw-agent-dual-mode-contract-2026-03-16.md`
- `testing-manual.md`

## 状态
- 最近更新：2026-04-07（T4.5 完成，专题恢复 `completed`）
- 当前阶段: completed
- 当前任务: 本专题已恢复 `completed`；后续风险与扩面工作转入 `PRD-WORLD_SIMULATOR-038` / `PRD-WORLD_SIMULATOR-037` 的 experimental follow-up。
- owner: `qa_engineer`
- 联审: `agent_engineer`、`runtime_engineer`、`viewer_engineer`、`qa_engineer`
- 发起建模: `producer_system_designer`
- 备注: 本专题的目标态没有变化，仍然是为 `agent_direct_connect` 当前 provider implementation=`openclaw_local_http` 收口“双轨 lane”的产品目标与执行边界。2026-04-06 formal review 提出的 launcher 可达性、dual-mode observation、provider handshake 与 fallback 审计链缺口，现已在 `TASK-WORLD_SIMULATOR-295~298` 中完成 remediation 与真实复采证；2026-04-07 复签结论见 `doc/world-simulator/llm/openclaw-agent-dual-mode-recertification-2026-04-07.md`。
- 备注补充: runtime live / viewer debug context 当前新增的 `capabilities`、`supported_action_sets` 与 compatibility status 表达的是本地执行 lane 期望遵守的 phase-1 contract 与 fallback 审计结果，不等价于 provider 实际 `/v1/provider/info` handshake 原样回显；真实 provider 兼容性真值仍以 launcher provider compatibility check 与 parity summary/raw 为准。
- 当前阻断:
  - 无。本专题已恢复 `completed`。
  - 后续非阻断项: `PRD-WORLD_SIMULATOR-038` 仍需继续处理 absolute wait latency 与更广 parity 样本；本轮 `headless_agent` 样本中的一次可恢复 `provider_unreachable` 记为 follow-up 观察项，而不是 reopen 本专题的 blocker。
