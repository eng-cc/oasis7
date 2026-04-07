# Local Provider vs 内置 Agent parity benchmark 协议（2026-03-12）

审计轮次: 1

## 目标
- 为 `PRD-WORLD_SIMULATOR-038` 冻结 builtin 与 `Local Provider(Local HTTP)` 的统一 fixture benchmark 协议，避免不同 provider 使用不同输入条件导致结果不可比。
- 定义 benchmark 运行时所需的固定字段、trace 收集要求、输出产物与失败签名分类。
- 为后续自动脚本实现提供稳定契约，使 `test_tier_required` 可以在 mock provider 与 builtin provider 上离线复用。

## 范围
- In Scope:
  - benchmark 输入包结构。
  - 运行约束（seed、timeout、采样次数、provider 标识）。
  - trace 汇总字段与 JSON/CSV 产物结构。
  - 错误码分类与 benchmark 结果状态。
- Out of Scope:
  - 不直接定义最终脚本实现语言或 CLI 细节。
  - 不包含真实 `Local Provider` 服务端实现。

## Benchmark 输入协议

### 1. 顶层运行字段
- `benchmark_run_id`: 本次对标批次 ID。
- `parity_tier`: `P0` / `P1` / `P2`。
- `scenario_id`: 取自 parity 场景矩阵。
- `fixture_id`: 固定 observation/seed 样本 ID。
- `provider_kind`: `builtin` / `provider_loopback_http` / `mock_provider`。
- `provider_version`: provider 自报版本。
- `adapter_version`: adapter 自报版本。
- `protocol_version`: benchmark 协议版本。
- `seed`: 固定随机种子。
- `timeout_ms`: 单轮 benchmark 的最大允许时长。
- `max_steps`: 单轮最大决策步数。

### 2. 场景输入字段
- `initial_world_snapshot_ref`: 输入世界快照引用。
- `observation_sequence_ref`: 固定 observation 序列引用。
- `goal_definition`: 当前样本的目标达成条件。
- `action_catalog_ref`: 动作白名单引用。
- `player_context_ref`: player/agent 绑定上下文引用。
- `memory_fixture_ref`: 预置短期/长期记忆样本引用（如适用）。

### 3. Provider 执行约束
- builtin 与 Local Provider 必须使用同一 `fixture_id`、`seed`、`timeout_ms`、`max_steps`。
- 若任一输入引用缺失，则该样本直接标记 `invalid_fixture`，不得进入正式对比结果。
- provider 不得绕过 benchmark 提供的动作白名单或额外读取未声明上下文。

## Trace 汇总字段

### 1. 每步 trace 记录
- `step_index`
- `provider_kind`
- `decision`
- `action_ref`
- `latency_ms`
- `error_code`
- `retry_count`
- `trace_present`
- `trace_message_count`
- `trace_tool_call_count`
- `context_drift_flag`

### 2. 单样本 summary
- `benchmark_run_id`
- `scenario_id`
- `fixture_id`
- `provider_kind`
- `status`: `passed` / `failed` / `blocked` / `invalid_fixture`
- `goal_completed`
- `completion_time_ms`
- `decision_steps`
- `invalid_action_count`
- `timeout_count`
- `recoverable_error_count`
- `fatal_error_count`
- `trace_completeness_ratio`
- `median_latency_ms`
- `p95_latency_ms`
- `notes`

### 3. 聚合 summary
- `parity_tier`
- `scenario_id`
- `provider_kind`
- `sample_count`
- `completion_rate`
- `invalid_action_rate`
- `timeout_rate`
- `recoverable_error_resolution_rate`
- `median_extra_wait_ms`
- `p95_extra_wait_ms`
- `trace_completeness`
- `context_drift_count`
- `benchmark_status`

## 产物结构
- `artifacts/<run_id>/raw/*.jsonl`: 每步 trace 原始记录。
- `artifacts/<run_id>/summary/<scenario_id>.<provider>.json`: 单场景聚合结果。
- `artifacts/<run_id>/summary/combined.csv`: builtin 与 Local Provider 并排对比表。
- `artifacts/<run_id>/summary/failures.md`: 失败签名与阻断项摘要。
- `artifacts/<run_id>/scorecard-links.md`: 对应主观评分卡和截图/trace 链接。

## 错误码分类
- `invalid_fixture`
- `provider_unreachable`
- `version_mismatch`
- `timeout`
- `invalid_action_schema`
- `action_rejected`
- `trace_missing`
- `context_drift`
- `session_cross_talk`
- `unclassified`

## 判定规则
- 单样本命中 `invalid_fixture` 时，不纳入 completion/latency 统计。
- 单样本命中 `session_cross_talk` 或 runtime 权威绕过时，直接记为 `blocked`。
- 聚合 summary 只在有效样本数满足最小样本数时输出 `benchmark_status`；否则为 `insufficient_data`。

## PRD-ID 映射
- `PRD-WORLD_SIMULATOR-038`: 统一 fixture benchmark、trace 汇总与聚合口径。
- `PRD-WORLD_SIMULATOR-037`: provider local HTTP 模式下的 benchmark 可执行性。
- `PRD-WORLD_SIMULATOR-036`: Decision Provider 契约字段复用。

## 风险与约束
- 若 `trace_present` 仅记录布尔值而缺具体 trace 字段计数，后续会难以追根因。
- 若 combined.csv 不并排输出 builtin/Local Provider，同层对比会增加误读风险。
- 若错误码粒度过粗，失败签名会失去行动指导意义。
