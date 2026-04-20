# oasis7 Runtime：WASM 可观测性与耗时指标设计

- 对应需求文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.project.md`

审计轮次: 1

## 1. 设计定位
本设计把 WASM 体系里“只能靠 ignored perf probe 和临时日志回答”的性能问题，收敛为正式 runtime 观测面。目标不是替代本地 benchmark，而是提供一条长期可复用的 `build -> executor -> router -> status -> summary` 证据链，让 release candidate、CI incident 与节点侧热点都用同一套字段归因。

## 2. 现状盘点

| 层级 | 当前实现事实 | 观测缺口 |
| --- | --- | --- |
| build suite | `tools/wasm_build_suite` 已输出 `source_hash/build_manifest_hash/wasm_hash/wasm_size_bytes` | 没有 `build_wall_ms/cargo_build_ms/canonicalize_ms`，无法判断 canonical build 成本 |
| executor | `compile_module_cached()` 已天然区分 memory hit、disk hit、compile miss | 这些路径没有累计 timing/counter snapshot，只有零散 `Instant` 与失败码 |
| router | 本地 perf probe 已能测 `parse_each_time -> prepared_once` | 结果只停留在 ignored test `eprintln!`，没有持续观测 surface |
| status | `/v1/chain/status` 已有 `storage` 与 `traffic` 共享 snapshot 模式 | 还没有 `wasm` section，无法在节点侧看到 executor/router/build 热点 |
| summary | 现有 triad traffic monitor 已能把 cumulative counter 转窗口 delta | wasm 还没有对应 summary 入口，无法持续输出 p50/p95/top hotspot |

## 3. 设计原则
- 原则-1：指标必须复用既有 `shared snapshot -> status payload -> external summary` 模式，不再平行创造另一套日志协议。
- 原则-2：wall-clock timing 只属于本地观测层，绝不能进入 deterministic world state、event log 或共识数据。
- 原则-3：默认 surface 只暴露 bounded counters、sums、buckets；模块级明细必须是 top-N 或 allowlist，而非全量 map。
- 原则-4：先解决“归因可回答”，再追求极致精细度；首期优先覆盖 build、compile/cache、executor call、router match 四类热点。

## 4. 目标态架构

```text
tools/wasm_build_suite
  -> build timing metadata / receipt
           |
           v
oasis7_wasm_executor -----+
  -> cache/compile/call   |
           |              |
           v              |
oasis7_wasm_router        |
  -> prepare/match timing |
           |              |
           +------> shared wasm metrics snapshot
                              |
                              v
                 /v1/chain/status.wasm
                              |
                              v
             repo-owned window summary script
                              |
                              v
                 summary.md / summary.json
```

关键变化：
- build receipt 从“只有 hash/size”升级为“hash/size + timing”。
- executor/router 从“一次性 perf probe”升级为“进程级累计 snapshot”。
- `/v1/chain/status` 从“storage/traffic only”升级为“storage/traffic/wasm 三面并列”。

## 5. 详细设计

### 5.1 数据面分层
- build 层：
  - 写入 `metadata.json` / `build-receipt.json`
  - 适合记录单次构建耗时
- runtime 层：
  - 维护 `SharedWasmMetricsSnapshot`
  - 适合记录节点生命周期内累计 counters/sums/buckets
- summary 层：
  - 读取 status samples
  - 适合计算窗口 delta、bucket 派生 p50/p95、top hotspot

这三层明确分工，避免“单次构建耗时”和“进程累计执行耗时”混在一个 schema 里。

### 5.2 Build Timing Contract
`tools/wasm_build_suite` 新增 timing 字段：
- `total_build_wall_ms`
- `cargo_build_ms`
- `canonicalize_ms`
- `hash_ms`
- `receipt_write_ms`
- 可选：`metadata_write_ms`

约束：
- 字段必须进入 metadata 与 receipt 的 machine-readable schema。
- `dry_run=true` 时允许部分字段为空，但必须显式区分，不得伪造 `0ms success`。
- timing 单位统一使用毫秒。

### 5.3 Runtime Snapshot Contract
新增 `WasmMetricsSnapshot`，建议与 `StorageMetricsSnapshot` 风格对齐：
- `observed_since_unix_ms`
- `metrics_available`
- `degraded_reason`
- `build`: 最近一次构建或导入的 bounded timing 摘要
- `executor`: 累计 counters/sums/buckets
- `router`: 累计 counters/sums/buckets

其中 `executor` 至少包括：
- `calls_total`
- `memory_cache_hits`
- `disk_cache_hits`
- `compile_misses`
- `failure_by_code`
- `compile_ms_total`
- `deserialize_ms_total`
- `instantiate_ms_total`
- `entrypoint_call_ms_total`
- `decode_ms_total`
- `call_wall_ms_buckets`

其中 `router` 至少包括：
- `prepare_subscriptions_ms_total`
- `match_filters_ms_total`
- `regex_compile_ms_total`
- `prepared_hits`
- `parse_fallbacks`
- `match_wall_ms_buckets`

### 5.4 Executor Instrumentation Points
`oasis7_wasm_executor` 的关键埋点：
- `compile_module_cached()`
  - memory cache lookup
  - disk cache load
  - deserialize compiled artifact
  - compile miss
  - serialize/store compiled artifact
- `call()`
  - instantiate
  - input write
  - entrypoint call
  - output read
  - output decode
  - total wall clock

设计约束：
- timing 记录在本地变量结束后统一更新 snapshot，减少锁持有时间。
- 失败路径也要记 timing 与 failure code，避免只统计成功样本。
- snapshot 更新必须使用短临界区；锁失败时仅打 `degraded_reason`，不影响主执行流程。

### 5.5 Router Instrumentation Points
`oasis7_wasm_router` 的关键埋点：
- `prepare_subscriptions()`
- `prepare_subscription_filters()`
- regex compile cache miss
- `prepared_module_subscribes_to_event/action()`

设计重点：
- router 默认不输出全量 `module_id -> timing` 明细。
- 若后续需要模块级热点，建议先提供 bounded `top_modules_by_match_ms`，并限制为 `top 10`。

### 5.6 Status Payload Exposure
`oasis7_chain_runtime` 新增：
- `status_payload.rs` 中的 `wasm: WasmMetricsSnapshot`
- 构建/刷新 `snapshot_wasm_metrics(...)`

语义与现有 storage/traffic 保持一致：
- 状态接口只读
- 返回 cumulative snapshot
- 外部脚本自行计算窗口 delta

不做的事：
- 不在 status 接口直接输出 p95
- 不在 status 接口直接输出全量原始时序
- 不在 status 接口暴露未裁剪的模块级 map

### 5.7 External Summary Script
新增 repo-owned summary 入口，职责是：
- 读取多次 `/v1/chain/status` 采样
- 识别 `observed_since_unix_ms` reset
- 计算 delta/s
- 从固定 buckets 派生 p50/p95
- 输出 `summary.md/json`

推荐最少输出：
- `delta_calls`
- `delta_compile_ms`
- `delta_timeout_count`
- `executor_p50/p95_call_ms`
- `router_p50/p95_match_ms`
- `top_hotspots`
- `degraded_reason`

### 5.8 Cardinality 与 Payload Guardrails
默认 guardrails：
- 禁止 `trace_id`
- 禁止原始 input/output bytes
- 禁止全量 `module_id -> metrics`
- 允许：
  - 全局累计 counters
  - 固定 buckets
  - bounded top-N

若后续需要更细维度：
- 必须显式 env-gated
- 必须有 payload budget
- 必须写明裁剪策略

## 6. 与现有体系的关系
- 与 `TASK-WORLD_RUNTIME-060` 的关系：
  - 不是替代 perf probe
  - 而是把其结论沉淀为长期可复核字段
- 与 `chain-status-traffic-metrics` 的关系：
  - 直接复用 shared snapshot + status payload 设计模式
- 与 `runtime-release-gate-metrics-template` 的关系：
  - 后续可把 `wasm.build/executor/router` 作为 runtime 候选指标的一部分

## 7. 回退与降级策略
- 若 metrics 初始化失败：
  - `metrics_available=false`
  - 写 `degraded_reason`
  - 继续允许 module 执行
- 若 status payload 裁剪命中：
  - 显式输出 `truncated=true` 或等价说明
- 若 summary 脚本遇到 reset：
  - 自动缩窗
  - 在 summary 中标记 `window_reset_detected=true`

## 8. 实施顺序建议
1. build suite timing schema
2. executor snapshot
3. status payload `wasm` section
4. summary 脚本
5. router snapshot 与 bounded top-N

原因：
- build/executor 是最高价值且埋点最明确的两段。
- status payload 需要先有统一 schema。
- router 明细容易受 cardinality 影响，放在后半段更稳。
