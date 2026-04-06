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
- 最近更新：2026-03-18
- 当前阶段: completed
- 当前任务: `none`（T4 已完成；后续转入 `PRD-WORLD_SIMULATOR-038` 的 builtin/OpenClaw parity 扩面采证）。
- owner: `qa_engineer`
- 联审: `agent_engineer`、`runtime_engineer`、`viewer_engineer`、`qa_engineer`
- 发起建模: `producer_system_designer`
- 备注: 本专题只定义 `agent_direct_connect` 当前 provider implementation=`openclaw_local_http` 下“双轨 lane”的产品目标与执行边界；它不替代 `PRD-WORLD_SIMULATOR-038` 的 parity 门禁，而是为 parity/回归/观战拆出各自清晰口径。2026-03-18 已将该分层直接回写到 `oasis7` operator 入口，随后又把非关键 UI/observer 细节拆到独立 reference，明确“agent 直连能跑不等于必须开 UI”，且主 skill 应优先暴露执行闭环。
- 当前阻断: `none`。T4 已基于 `2026-03-17` 的真实 `headless_agent` / `player_parity` 双样本完成 QA / producer 对照采证，并冻结默认模式：`headless_agent` 作为 CI / server / 回归默认，`player_parity` 作为体验对照与准入门禁。详见 `doc/testing/openclaw-dual-mode-t4-blocker-2026-03-16.md`。
