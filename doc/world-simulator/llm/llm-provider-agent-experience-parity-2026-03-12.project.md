# Local Provider 与内置 Agent 体验等价（parity）验收方案（2026-03-12）项目管理文档

- 对应设计文档: `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.design.md`
- 对应需求文档: `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.prd.md`

审计轮次: 2

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-038) [test_tier_required]: 完成 `Local Provider` 与内置 agent 体验等价（parity）专题 PRD / Design / Project 建模，并回写模块主文档、索引与 devlog。
- [x] T1 (PRD-WORLD_SIMULATOR-038) [test_tier_required]: 冻结 `P0/P1/P2` 场景集、评分项、通过线与阻断线，并新增 parity 场景矩阵与评分卡模板。
- [x] T2 (PRD-WORLD_SIMULATOR-038) [test_tier_required]: 为 builtin 与 provider 冻结统一 fixture benchmark 协议、trace 汇总字段与分数聚合模板。
- [x] T3 (PRD-WORLD_SIMULATOR-038) [test_tier_required]: 将 `Local Provider(Local HTTP)` 专题与 `Decision Provider` 专题的实施任务改挂到 parity 目标，确保“接通”不等于“完成”。
- [x] T4 (PRD-WORLD_SIMULATOR-038) [test_tier_full]: 完成真实 `Local Provider(Local HTTP)` 的 `P0` parity 对标试玩，输出 QA/producer 双签结论。
- [x] T4A (PRD-WORLD_SIMULATOR-038) [test_tier_required]: 冻结分层 latency gate（行为等价 hard gate + `latency_class`），并基于 `fix3` 证据回写 `behavior_parity_pass / latency_class B / keep experimental` 追加结论。
- [ ] T5 (PRD-WORLD_SIMULATOR-038) [test_tier_full]: 在保持 `experimental` 的前提下推进 `P1`/`P2` 行为等价扩面；只有当 Local Provider 达到 `latency_class A` 时，才允许作为默认体验或扩大覆盖范围。

## 依赖
- `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.prd.md`
- `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.prd.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-scenario-matrix-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-score-card-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-benchmark-protocol-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-aggregation-template-2026-03-12.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `doc/world-simulator/llm/provider-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md`
- `doc/testing/provider-agent-parity-p0-t4-closure-2026-03-17.md`

## 状态
- 最近更新：2026-03-17
- 当前阶段: `T4A completed / T5 gated`
- 当前任务: 冻结分层 latency gate，维持 `fix3` 的 `behavior_parity_pass / latency_class B / keep experimental` 口径，并继续压缩等待时延直至达到 `latency_class A`
- owner: `producer_system_designer`
- 联审: `qa_engineer`、`agent_engineer`、`viewer_engineer`、`runtime_engineer`
- 发起建模: `producer_system_designer`
- 备注: 本专题将“体验等价”提升为上线门禁；后续若 `Local Provider` 未达到行为等价或命中 `latency_class C`，只允许保留在 `experimental`，不得标记为默认体验。

- T4 进展备注: 已落地 `crates/oasis7/src/bin/oasis7_provider_parity_bench.rs` 与 `scripts/provider-parity-p0.sh`，用于按 `PRD-WORLD_SIMULATOR-038` benchmark 协议输出 `raw/*.jsonl`、单样本 summary、聚合 `combined.csv`、`failures.md` 与 `scorecard-links.md`；`PRD-WORLD_SIMULATOR-040` 已于 `2026-03-17` 完成 `headless_agent` vs `player_parity` 双模式默认策略冻结，真实 builtin/Local Provider 对标现在成为唯一剩余 T4 缺口。
- T4 口径补充: parity harness 已新增 `--agent-provider-profile` 并默认固定到 `oasis7_p0_low_freq_npc`（兼容旧别名 `oasis7_p0_low_freq_npc`）；`DecisionRequest.agent_profile`、summary provider 信息与批处理脚本现已保留该 profile，便于 QA/producer 确认样本不是在“未知通用 skill”下跑出来的结果。
- T4 主链路补充: 产品 launcher 主链路现已可把同一 profile 透传到真实 runtime live，因此后续 builtin/Local Provider 双边 parity 样本可直接复用 GUI launcher 配置，而不必只依赖 bench harness。
- T4 原始结论（2026-03-17 / `t4d`）: 真实批次 `provider_builtin_parity_20260317_t4d` 显示 builtin `completion_rate=0%`、Local Provider `completion_rate=100%`，`P0-001` completion gap 为 `100pp`，当前 parity 结论为 `failed`，必须保持 `experimental`；详见 `doc/testing/provider-agent-parity-p0-t4-closure-2026-03-17.md`。
- T4 后续修复进展（2026-03-17 / `fix2`）: `agent_engineer` 已在 `oasis7_provider_parity_bench` 为 builtin lane 补 `P0-001` patrol guardrail，并将 bench / `scripts/provider-parity-p0.sh` 默认 connect-timeout 对齐到 `15000ms`；真实批次 `provider_builtin_parity_20260317_fix2` 已显示 builtin `completion_rate=100%`、`move_agent=4`，但 Local Provider 样本仍为 `timeout=4` / `completion_rate=0%`，因此整体状态仍为 `T5 gated`。
- T4 后续修复进展（2026-03-17 / `fix3`）: `agent_engineer` 已为 `oasis7_provider_local_bridge` 增加 `gateway call agent` timeout 到 `provider agent --local` 的 fallback，并用稳定 hash session-id 保持会话隔离；真实批次 `provider_builtin_parity_20260317_fix3` 已显示 builtin/Local Provider 均为 `completion_rate=100%`、`timeout_rate=0%`、`move_agent=4`。
- T4A 追加结论（2026-03-17 / 审计轮次 2）: `fix3` 批次中 builtin `median_extra_wait_ms=9900`、`p95_extra_wait_ms=10597`，Local Provider `median_extra_wait_ms=13957`、`p95_extra_wait_ms=14062`；相对 gap 为 `median=4057ms`、`p95=3465ms`，已满足行为等价硬门禁，但 Local Provider 仅达到 `latency_class B`，因此允许继续 `experimental` / 受限试点，不允许默认启用。
