# OpenClaw 双轨模式 T4.5 复签结论（2026-04-07）

- owner: `qa_engineer`
- 联审: `producer_system_designer`
- 关联 PRD: `PRD-WORLD_SIMULATOR-040`、`PRD-WORLD_SIMULATOR-038`
- 关联任务:
  - `doc/world-simulator/llm/llm-openclaw-agent-dual-mode-2026-03-16.project.md` T4.5
  - `TASK-WORLD_SIMULATOR-298`
- 结论状态:
  - `PRD-WORLD_SIMULATOR-040`: `pass / restore completed`
  - `PRD-WORLD_SIMULATOR-038`: `unchanged / keep experimental`

## 1. 复签结论
- 2026-04-06 formal review 提出的三项 remediation 验收目标，本轮已经可以按“真实 dual-mode 样本 + 已落地 required-tier 回归”联合复签：
  - 入口可达：`TASK-WORLD_SIMULATOR-295` 已证明 client launcher 会真实透传 `OpenClaw execution mode`；本轮 `player_parity` / `headless_agent` 真实样本也都在 summary 中记录到了对应 `mode` 与 `environment_class`，说明请求 lane 没有再静默塌缩到单一路径。
  - observation 分层：`TASK-WORLD_SIMULATOR-296` 的 fixture diff / negative tests 仍是“`player_parity` 不泄露 headless-only 真值”的权威证据；本轮真实样本则补上了 remediation 后的 live run 佐证，证明双 lane 不再只是 metadata 标签。
  - fallback 可审计：provider `/info` 和本轮 parity summary 已共同输出 `capabilities`、`supported_action_sets`、`compatibility_status` 与 `fallback_reason`，可直接判断当前运行是否 `ready`、是否发生 fallback。
- 因此，本专题可以恢复为 `completed`；但这次复签不改变 `PRD-WORLD_SIMULATOR-038` 的实验态结论，OpenClaw 绝对等待时延仍高于默认放行线，且本轮 `headless_agent` 样本仍出现一次可恢复 `provider_unreachable` 抖动。
- `restore completed` 在这里的含义是：2026-04-06 formal review reopen 的四项 contract / reachability / audit gap 已经被修正并经过真实样本复核；它不等于“仅凭本轮 `samples=1` 就重新统计证明了全部成功率/复现率门槛”。涉及成功率、复现率与更广样本稳定性的判断，继续以本专题既有历史证据与后续 soak / parity 跟踪共同维护。

## 2. 批次与环境
- 执行日期: `2026-04-07`
- OpenClaw CLI: `OpenClaw 2026.3.31 (213a704)`
- Gateway 健康检查: `curl -sS http://127.0.0.1:18789/health` 返回 `{"ok":true,"status":"live"}`
- Bridge 健康检查: `curl -sS http://127.0.0.1:5841/v1/provider/health | jq .`
  - `ok=true`
  - `status="ok"`
- Provider info: `curl -sS http://127.0.0.1:5841/v1/provider/info | jq .`
  - `provider_id=openclaw_local_bridge`
  - `supported_action_sets=[wait, wait_ticks, move_agent, speak_to_nearby, inspect_target, simple_interact]`
  - `capabilities=[decision, feedback, loopback_only, agent:oasis7_openclaw_agent]`

## 3. 执行批次
- `headless_agent`
  - command: `env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/tmp/oasis7-task298-target bash scripts/openclaw-parity-p0.sh --openclaw-only --samples 1 --ticks 4 --timeout-ms 15000 --openclaw-base-url http://127.0.0.1:5841 --openclaw-connect-timeout-ms 15000 --openclaw-agent-profile oasis7_p0_low_freq_npc --execution-mode headless_agent`
  - benchmark_run_id: `openclaw_parity_20260407_112652`
  - summary: `output/openclaw_parity/openclaw_parity_20260407_112652/summary/P0-001.openclaw_local_http.json`
  - sample summary: `output/openclaw_parity/openclaw_parity_20260407_112652/samples/openclaw_local_http/sample_1/summary/P0-001.openclaw_local_http.json`
  - raw trace: `output/openclaw_parity/openclaw_parity_20260407_112652/samples/openclaw_local_http/sample_1/raw/P0-001_sample_1.openclaw_local_http.jsonl`
- `player_parity`
  - command: `env -u RUSTC_WRAPPER CARGO_TARGET_DIR=/tmp/oasis7-task298-target bash scripts/openclaw-parity-p0.sh --openclaw-only --samples 1 --ticks 4 --timeout-ms 15000 --openclaw-base-url http://127.0.0.1:5841 --openclaw-connect-timeout-ms 15000 --openclaw-agent-profile oasis7_p0_low_freq_npc --execution-mode player_parity`
  - benchmark_run_id: `openclaw_parity_20260407_112747`
  - summary: `output/openclaw_parity/openclaw_parity_20260407_112747/summary/P0-001.openclaw_local_http.json`
  - sample summary: `output/openclaw_parity/openclaw_parity_20260407_112747/samples/openclaw_local_http/sample_1/summary/P0-001.openclaw_local_http.json`
  - raw trace: `output/openclaw_parity/openclaw_parity_20260407_112747/samples/openclaw_local_http/sample_1/raw/P0-001_sample_1.openclaw_local_http.jsonl`

## 4. 核心对照结果
| lane | mode / env | completion_rate | invalid_action_rate | timeout_rate | recoverable_error_resolution_rate | median_extra_wait_ms | p95_extra_wait_ms | fallback_reason | compatibility_status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `headless_agent` | `headless_agent` / `headless_linux` | `1.0` | `0.0` | `0.0` | `1.0` | `8798` | `9105` | `null` | `ready` |
| `player_parity` | `player_parity` / `player_parity_linux` | `1.0` | `0.0` | `0.0` | `1.0` | `8697` | `9361` | `null` | `ready` |

补充观察：
- 两条 lane 都保持 `goal_completed=true`、`decision_steps=4`、`trace_completeness=1.0`、`context_drift_count=0`。
- `headless_agent` 样本出现一次可恢复 `provider_unreachable`，最终在同批次内恢复并完成目标；`player_parity` 未复现该抖动。
- 两条 lane 的 summary 都保留了 remediation 要求的 `mode`、`observation_schema_version`、`action_schema_version`、`environment_class` 与 `fallback_reason` 字段。
- 两条 lane 的 sample summary 都在 `provider` 字段里携带了 `capabilities`、`supported_action_sets` 与 `compatibility_status=ready`，说明本轮不是“只知道 provider 活着”，而是拿到了 phase-1 contract 层面的兼容性判定。

## 5. QA 结论
- 结论: `pass`
- 判定:
  - `PRD-WORLD_SIMULATOR-040` 关注的是双 lane 的产品边界、可达性、观测分层和可审计性，而不是 builtin/OpenClaw 的体验等价。
  - 结合 `TASK-WORLD_SIMULATOR-295~297` 的 required-tier 定向回归与本轮真实样本，当前已能证明 remediation 目标成立，因此本专题可恢复 `completed`。
  - 本轮 `completed` 复签针对的是 remediation reopen 后的 contract 真值，不把 `samples=1` 误表述成一次新的统计稳定性认证。
- residual risk:
  - `headless_agent` 样本的首步仍记录到一次 `provider_unreachable`，虽然最终被恢复且未触发 lane fallback，但这是值得继续观察的失败签名。
  - 本轮绝对等待仍处于约 `8.7s~9.4s` 量级，不满足 `PRD-WORLD_SIMULATOR-038` 默认放行所需的 `latency_class A`。
  - 本轮没有把 source-tree `play` 冷启动编译结果纳入 gate 证据；launcher reachability 继续以 `TASK-WORLD_SIMULATOR-295` 已通过的定向回归为主，本轮真实样本负责确认 remediation 后的 lane 真值与审计字段已进入 live output。

## 6. Producer 结论
- 结论: 同意将 `PRD-WORLD_SIMULATOR-040` 恢复为 `completed`。
- 默认模式策略保持不变:
  - `headless_agent`: CI / server / 自动回归默认 lane
  - `player_parity`: QA / producer 体验对照与准入 lane
  - `debug_viewer`: observer-only 旁路层
- 与 `PRD-WORLD_SIMULATOR-038` 的关系:
  - 本轮复签不改变 `behavior_parity_pass / latency_class B / keep experimental` 的现行口径。
  - 只有当更广样本下的 builtin/OpenClaw parity 与绝对等待时延一并收口后，才允许重新讨论默认启用或扩大覆盖范围。

## 7. 后续动作
- 将 `PRD-WORLD_SIMULATOR-040` 项目状态恢复为 `completed`，并把 `TASK-WORLD_SIMULATOR-298` 标记完成。
- 把 OpenClaw 后续风险收口到 `PRD-WORLD_SIMULATOR-038` / `PRD-WORLD_SIMULATOR-037`：
  - 继续压缩 absolute wait latency。
  - 继续扩面 builtin/OpenClaw parity 样本。
  - 若 `provider_unreachable` 在 `headless_agent` soak 中持续复现，升级为独立 QA follow-up，而不是重新打开双轨专题本身。
