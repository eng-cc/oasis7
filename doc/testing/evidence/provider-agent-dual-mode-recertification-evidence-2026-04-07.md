# Local Provider 双轨模式复签 repo-owned 证据（2026-04-07）

审计轮次: 1

## Meta
- 关联专题: `PRD-WORLD_SIMULATOR-040`、`PRD-WORLD_SIMULATOR-038`
- 关联任务: `TASK-WORLD_SIMULATOR-298`、`TASK-WORLD_SIMULATOR-306`
- 责任角色: `producer_system_designer`
- 协作角色: `qa_engineer`
- 当前结论: `pass / restore completed for PRD-WORLD_SIMULATOR-040; keep experimental for PRD-WORLD_SIMULATOR-038`
- 目标: 把 2026-04-07 dual-mode 复签中的关键数值、判定边界与 residual risk 固化为 repo-owned 审计摘要，让 reviewer 不依赖 `.gitignore` 下的 `output/provider_parity/...` 也能复核为什么 `PRD-WORLD_SIMULATOR-040` 可以保持 `completed`，同时为什么 `PRD-WORLD_SIMULATOR-038` 仍不能默认启用。

## 最终结论
- `PRD-WORLD_SIMULATOR-040`:
  - 可维持 `pass / restore completed`
  - 含义仅限于 dual-mode contract、launcher reachability、observation 分层与 provider fallback/audit chain 的 remediation 已完成并被真实样本复核
- `PRD-WORLD_SIMULATOR-038`:
  - 维持 `behavior_parity_pass / latency_class B / keep experimental`
  - 含义是行为等价硬门禁已过，但绝对等待时延仍未达到默认启用所需的 `latency_class A`
- 因此，当前正式产品口径必须同时成立:
  - dual-mode 主题可以是 `completed`
  - Local Provider 仍不得默认启用，也不得把这轮复签表述成 release-grade default-enable 认证

## 批次信息
- 执行日期: `2026-04-07`
- Local Provider CLI: `Local Provider 2026.3.31 (213a704)`
- Gateway 健康检查: `{"ok":true,"status":"live"}`
- Bridge 健康检查:
  - `ok=true`
  - `status="ok"`
- Provider info:
  - `provider_id=provider_local_bridge`
  - `supported_action_sets=[wait, wait_ticks, move_agent, speak_to_nearby, inspect_target, simple_interact]`
  - `capabilities=[decision, feedback, loopback_only, agent:oasis7_provider_agent]`

## 核心指标摘要
| lane | completion_rate | invalid_action_rate | timeout_rate | recoverable_error_resolution_rate | median_extra_wait_ms | p95_extra_wait_ms | fallback_reason | compatibility_status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `headless_agent` | `1.0` | `0.0` | `0.0` | `1.0` | `8798` | `9105` | `null` | `ready` |
| `player_parity` | `1.0` | `0.0` | `0.0` | `1.0` | `8697` | `9361` | `null` | `ready` |

补充观察:
- 两条 lane 都保持 `goal_completed=true`、`decision_steps=4`、`trace_completeness=1.0`、`context_drift_count=0`
- `headless_agent` 样本记录到一次可恢复 `provider_unreachable`，但在同批次内恢复并完成目标
- 两条 lane 的 summary 都带有 `mode`、`observation_schema_version`、`action_schema_version`、`environment_class` 与 `fallback_reason`
- 两条 lane 的 sample summary 都带有 `capabilities`、`supported_action_sets` 与 `compatibility_status=ready`

## 为什么这次是 completed，但不是 default-enable
- `PRD-WORLD_SIMULATOR-040` 的 scope 是双轨 execution lane 的产品边界、reachability、观测分层与可审计性
- 2026-04-06 formal review reopen 的缺口是:
  - client launcher lane 可达性
  - dual-mode observation 真实分层
  - provider handshake / fallback 审计链
  - 证据可复核性
- 上述缺口在 `TASK-WORLD_SIMULATOR-295~298` 与 `TASK-WORLD_SIMULATOR-306` 后已被补齐，所以 dual-mode 主题可恢复并保持 `completed`
- 但 `PRD-WORLD_SIMULATOR-038` 控制的是 provider 与 builtin 的体验等价和默认启用门槛；该门槛仍要求 latency 继续收敛
- 当前 absolute wait 仍约在 `8.7s~9.4s`，所以虽然 `behavior_parity_pass` 已成立，默认启用资格仍未成立

## Repo-owned 与 raw artifact 的关系
- 本文档是 repo-owned 审计摘要，提供 reviewer 所需的核心数字、边界与结论
- ignored raw artifacts 仍保留在以下路径，供需要 drill-down 的人复查:
  - `output/provider_parity/provider_parity_20260407_112652/...`
  - `output/provider_parity/provider_parity_20260407_112747/...`
- reviewer 若只看 git diff，也应能仅凭本文与 `doc/world-simulator/llm/provider-agent-dual-mode-recertification-2026-04-07.md` 理解当前正式口径

## 当前产品表达约束
- runtime live / software-safe 必须把两类信息分开展示:
  - execution lane 期望 metadata: `mode/schema/environment/fallback` 与 phase-1 contract `compatibility_status`
  - provider 实际 readiness truth: `provider_check_status/source/fallback_reason/capabilities/supported_action_sets/error`
- 这样做的原因是: lane metadata 解释“当前想按什么 contract 运行”，actual provider check 才解释“当前 provider 实际有没有 ready”

## 残余风险与后续项
- `headless_agent` 的单次可恢复 `provider_unreachable` 仍需继续观察；若在 soak 中持续复现，应升级为独立 follow-up
- Local Provider absolute wait latency 仍未达到默认启用线，后续仍需继续压缩 prompt/调用链开销
- `PRD-WORLD_SIMULATOR-039` 的 formal Web gameplay 仍待在具备可用 LLM provider 的环境里复采；这与本双轨 completed 结论无冲突，但会继续限制 release claim
