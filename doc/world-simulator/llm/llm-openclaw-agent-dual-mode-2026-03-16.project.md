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
- [x] T4.4 (PRD-WORLD_SIMULATOR-040) [test_tier_required]: 由 `viewer_engineer` / `runtime_engineer` 为 launcher probe、runtime live snapshot、viewer debug context 与 parity summary 补 `capabilities/supported_action_sets` 兼容校验与 `fallback_reason` 透传，收口 `ready/degraded/incompatible` 判定与 `agent_direct_connect` alias fallback 审计链。
- [ ] T4.5 (PRD-WORLD_SIMULATOR-040) [test_tier_full]: 由 `qa_engineer` / `producer_system_designer` 在 T4.2~T4.4 完成后重跑 dual-mode 真实采证，并重新判断本专题是否可恢复 `completed`。

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
- 最近更新：2026-04-07（T4.4 完成）
- 当前阶段: remediation_reopened
- 当前任务: `T4.4` 已完成；进入 `T4.5` dual-mode 重采证与 producer/QA 复签。
- owner: `qa_engineer`
- 联审: `agent_engineer`、`runtime_engineer`、`viewer_engineer`、`qa_engineer`
- 发起建模: `producer_system_designer`
- 备注: 本专题的目标态没有变化，仍然是为 `agent_direct_connect` 当前 provider implementation=`openclaw_local_http` 收口“双轨 lane”的产品目标与执行边界；但 2026-04-06 formal review 已确认当前实现尚未完整兑现 `PRD-WORLD_SIMULATOR-040` 的 launcher 可达性、dual-mode observation、provider handshake 与 fallback 审计链要求，因此 project 状态从“已收口”调整为“需补 remediation 后再复签”。详见 `doc/world-simulator/llm/llm-openclaw-agent-direct-connect-review-2026-04-06.md`。
- 备注补充: runtime live / viewer debug context 当前新增的 `capabilities`、`supported_action_sets` 与 compatibility status 表达的是本地执行 lane 期望遵守的 phase-1 contract 与 fallback 审计结果，不等价于 provider 实际 `/v1/provider/info` handshake 原样回显；真实 provider 兼容性真值仍以 launcher probe 与 parity summary/raw 为准。
- 当前阻断:
  - 已解除: client launcher 现已显式暴露并透传 `OpenClaw execution mode`，且与 `oasis7_game_launcher` / operator 默认 connect timeout 统一到 `15000ms`
  - 已解除: dual-mode request 已切到 provider-facing observation adapter，`player_parity` 不再携带 `local_navigation_graph/interaction_targets` 等 headless-only 字段；bridge 对 `unsupported_schema_version` 与 `mode_observation_mismatch` 现已返回结构化失败，fixture diff 定向测试已落地
  - 已解除: launcher probe 现会基于 `capabilities/supported_action_sets` 给出 `ready/degraded/incompatible`，runtime live debug context 与 parity summary/raw 已写入 `capabilities`、`supported_action_sets`、compatibility status 与 `fallback_reason`，`agent_direct_connect` alias 也会留下结构化 fallback 审计
  - 当前剩余阻断: 仅剩 `T4.5` 要求的真实 dual-mode 复采证与 producer/QA 重新判断专题是否可恢复 `completed`
