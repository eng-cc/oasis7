# oasis7 Runtime：三节点完整监控体系（设计文档）

- 对应需求文档: `doc/p2p/node/node-triad-observability-stack-2026-04-23.prd.md`
- 对应项目管理文档: `doc/p2p/node/node-triad-observability-stack-2026-04-23.project.md`

审计轮次: 2

## 设计目标
- 复用现有 triad snapshot / traffic / wasm 采样，不改写现有 contract。
- 新增 host/process 采样层，把 CPU、load、memory、storage、systemd、runtime process 接入同一套 evidence。
- 新增 merged summary helper，把四类输入合并成一份 triad 真值，并基于 raw `status.json` 输出模块级 breakdown 与优化候选。

## 方案结构
1. `scripts/p2p-real-env-node-host-sample.sh`
   - 单机 helper。
   - 输入：`service`、`storage_path`。
   - 输出：shell-safe `key=value` 采样结果。
   - 运行位置：本机直接执行；远端通过 SSH `bash -s` 输送脚本内容执行。

2. `scripts/p2p-real-env-host-monitor.sh`
   - triad host/process monitor。
   - 负责当前 real-env triad（物理上本机 + 2 ECS，artifact label 以 `local_node / sequencer_ecs / storage_ecs` 为准，历史 `observer_local` 仅作兼容别名，但 runtime role 以 sampled status 为准）的周期采样。
   - 产物：`samples.ndjson`、`summary.json`、`summary.md`。

3. `scripts/p2p-real-env-host-summary.py`
   - 读取 host monitor history。
   - 输出 latest/peaks/status/alerts。
   - 规则：
     - `runtime_cpu_core_ratio >= 0.75` -> `runtime_cpu_hot`
     - `loadavg_1m / cpu_cores >= 1.0` -> `host_load_hot`
     - `mem_available_percent < 15` -> `memory_available_low`
     - `storage_used_percent >= 85` -> `storage_usage_high/critical`

4. `scripts/p2p-real-env-observability-monitor.sh`
   - triad 总控脚本。
   - 顺序执行：
     1. `p2p-real-env-triad-snapshot.sh`
     2. `p2p-real-env-host-monitor.sh`
     3. `p2p-real-env-traffic-monitor.sh`
     4. 从 snapshot `status.json` 提取 per-node wasm sample dir
     5. `oasis7-node-wasm-metrics-monitor.sh`
     6. 把 per-node raw `status.json` 与各类 summary 一并传给 `p2p-real-env-observability-summary.py`

5. `scripts/p2p-real-env-observability-summary.py`
   - 读取 snapshot/host/traffic/wasm summaries + per-node raw `status.json`。
   - 输出 triad merged summary。
   - 判定规则：
     - `snapshot.claim_status != pass_candidate` -> `overall.status=blocked`
     - `snapshot.claim_status == pass_candidate` 且 host 有资源告警 -> `pass_with_resource_alerts`
     - `snapshot.claim_status == pass_candidate` 且 host 无资源告警但存在模块级告警 -> `pass_with_module_alerts`
     - 否则 -> `pass_candidate`
   - 模块层：
     - `host_runtime`: 复用 host summary 的 CPU/load/memory/storage/service 状态。
     - `consensus/observability/replication/storage/reward_runtime/transactions/p2p_reachability`: 直接解析 runtime `/v1/chain/status` 顶层字段。
     - `wasm_executor_router`: 结合 raw `wasm` 字段与 `oasis7-node-wasm-metrics-monitor.sh` 的窗口摘要。
     - `traffic_control_plane`: 结合 traffic window 中的 payload/wire/control-plane 指标。
     - `p2p_reachability`: 消费 `/v1/chain/status.observability` 中由 runtime 投影的 bounded reachability/path summary；若字段未上报，输出 `not_reported`，不得在 summary helper 内重新推导 canonical path truth。
   - optimization candidates：
     - 允许跨模块拼接信号，例如 `runtime_cpu_hot + control_plane_wire_share_high` -> `libp2p_control_plane_churn`
     - 候选必须附 `evidence` 与 `suggested_optimizations`，避免只给口头判断。

## Reachability Path Summary Extension
本扩展对齐 `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md` 的 `PeerReachabilityContract`。triad observability 只消费 status projection，不拥有 reachability contract。

建议 summary shape：

| 字段 | 说明 | 边界 |
| --- | --- | --- |
| `selected_path_kind` | 当前 selected path kind，例如 `direct / hole_punched / relay / unknown` | 由 runtime status 提供；report 不重新判定 |
| `selected_path_age_ms` | 当前 selected path 在窗口内保持时长 | 可为 node-level summary；不要求 per-peer time series |
| `path_transition_counters` | `direct_to_relay / relay_to_direct / direct_to_hole_punched / hole_punched_to_relay` | bounded counter；不携带 raw peer IDs |
| `active_path_mix` | active direct/hole-punched/relay counts | 可用于判断 relay 依赖，但不是 public reachability claim |
| `recent_fallback_reason` | bounded enum，例如 `direct_failed / relay_reserved / path_stale / policy_override / unknown` | 禁止自由文本无限扩张 |
| `reachability_confidence` | `observed_direct / relay_reserved / punched_recently / proxy_only / manual_lab_required / not_reported` | 只用于 operator diagnosis；release claim 回到 matrix gate |

实现约束：
- summary helper 不读取 libp2p internals 或重新解析 peer records；它只消费 status JSON。
- 不新增独立 net-report endpoint；如果未来需要更丰富诊断，先扩展 `/v1/chain/status.observability`。
- 不导出 unbounded peer-id labels；peer 细节只允许 bounded debug artifact 或 top-N snapshot。
- byte split 必须保留现有 traffic scope 说明，不能声明 NIC-level overhead。

## 输出目录约定
- `<out-dir>/<run-id>/snapshot/`
- `<out-dir>/<run-id>/host/`
- `<out-dir>/<run-id>/traffic/`
- `<out-dir>/<run-id>/wasm/<label>/`
- `<out-dir>/<run-id>/report/latest_summary.{json,md}`
- `<out-dir>/latest_summary.{json,md}`

## 取舍说明
- 不直接修改现有 snapshot/traffic/wasm 的 summary contract，避免已有 evidence 断裂。
- host/process 监控独立成子脚本，而不是塞进 snapshot，便于单独验证资源问题。
- merged summary 集中在 Python helper，避免 shell/jq 多处复制同一套判定逻辑。
- 模块级 breakdown 也放在 merged summary helper 内实现，而不是散落到多个 node-specific 报表，避免阈值和 candidate 规则漂移。
