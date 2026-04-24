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
   - 负责 local observer + 2 ECS 的周期采样。
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
   - optimization candidates：
     - 允许跨模块拼接信号，例如 `runtime_cpu_hot + control_plane_wire_share_high` -> `libp2p_control_plane_churn`
     - 候选必须附 `evidence` 与 `suggested_optimizations`，避免只给口头判断。

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
