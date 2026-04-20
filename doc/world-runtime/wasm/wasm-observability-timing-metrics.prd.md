# oasis7 Runtime：WASM 可观测性与耗时指标

- 对应设计文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.design.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 当前 WASM 体系的性能证据分散在 `build receipt`、本地 ignored perf probe 与零散 status 字段里，无法持续回答“慢在构建、缓存、执行还是路由过滤”。当 release regression、CI 漂移或节点侧热点出现时，owner 仍要靠临时日志和本地复现拆解。
- Proposed Solution: 为 `tools/wasm_build_suite`、`oasis7_wasm_executor`、`oasis7_wasm_router` 建立统一的 cumulative observability snapshot，并通过 `/v1/chain/status.wasm` 与 repo-owned summary 脚本输出稳定的 timing / cache / failure 指标，让性能归因进入正式 runtime 观测面。
- Success Criteria:
  - SC-1: `/v1/chain/status.wasm` 必须稳定暴露 `build/executor/router` 三组 machine-readable 指标，并与节点生命周期累计同步。
  - SC-2: 执行器必须区分 `memory cache hit`、`disk cache hit`、`compile miss` 三条路径，并提供对应的 wall-clock timing 聚合。
  - SC-3: build suite 必须输出 `total_build_wall_ms` 与至少 3 个阶段耗时字段，确保 canonical build 不再只有 hash/size 而无 cost 证据。
  - SC-4: router 必须提供 prepared subscription 与 parse-each-time 的 timing 对比入口，保证 `TASK-WORLD_RUNTIME-060` 的优化收益能持续复核。
  - SC-5: 指标设计不得把 `trace_id`、原始 payload 或无界 `module_id` 明细直接写入默认 status payload，默认口径需满足 bounded cardinality 与 deterministic isolation。

## 2. User Experience & Functionality
- User Personas:
  - `wasm_platform_engineer`: 需要持续判断热点在 build、compile cache、executor 还是 router，而不是每次重新插桩。
  - `runtime_engineer`: 需要将 `/v1/chain/status` 中的 WASM timing 与 storage/traffic 一起纳入候选与 incident 复核。
  - `qa_engineer`: 需要把 perf regressions 收敛为固定验收字段，而不是依赖临时 perf probe 日志截图。
  - 节点运营者: 需要知道节点当前 WASM 热点是冷启动编译、磁盘反序列化还是持续执行超时。
- User Scenarios & Frequency:
  - release 前复核：每次涉及 `tools/wasm_build_suite`、`oasis7_wasm_executor`、`oasis7_wasm_router` 或 `/v1/chain/status` 结构改动时执行。
  - CI / candidate incident：当 canonical build、executor timeout 或 router filter regression 被怀疑时执行。
  - 节点侧巡检：节点启动、模块更新或负载异常后按需查看 `/v1/chain/status.wasm` 与窗口 summary。
  - 热点回归验证：每次热路径优化后执行，确认收益进入稳定观测面而非只停留在一次性 perf probe。
- User Stories:
  - PRD-WORLD_RUNTIME-036: As a `wasm_platform_engineer` / `runtime_engineer` / `qa_engineer`, I want the WASM build, executor, and router paths to emit bounded cumulative timing metrics and status snapshots, so that hotspot attribution no longer depends on ad hoc logs or ignored local perf probes.
- Critical User Flows:
  1. `build suite 构建模块 -> 记录 total/cargo/canonicalize/receipt timings -> 写入 metadata/receipt -> summary 脚本归档`
  2. `runtime 执行 module call -> 区分 memory cache hit / disk cache hit / compile miss -> 记录 compile/instantiate/call/decode timings -> 聚合到 executor snapshot`
  3. `router 准备订阅或匹配过滤器 -> 记录 prepare/match timing -> 更新 bounded counters/buckets -> 暴露给 status payload`
  4. `节点轮询 /v1/chain/status -> 读取 wasm snapshot -> 外部脚本做窗口 delta / p50 / p95 / top hotspot 汇总 -> 输出 summary.md/json`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| build timing snapshot | `total_build_wall_ms`、`cargo_build_ms`、`canonicalize_ms`、`hash_ms`、`receipt_write_ms`、`wasm_size_bytes` | canonical build 完成后写入 metadata/receipt；dry-run 不写真实 timing | `pending -> built -> reported` | 默认按模块构建完成时间写入；summary 按模块与 runner 汇总 | 仅 canonical build / verify 入口可写 |
| executor cumulative snapshot | `calls_total`、`memory_cache_hits`、`disk_cache_hits`、`compile_misses`、`compile_ms_total`、`deserialize_ms_total`、`instantiate_ms_total`、`entrypoint_call_ms_total`、`decode_ms_total`、`failure_by_code`、`call_wall_ms_buckets` | 每次 `ModuleSandbox::call` 更新累计计数与固定 bucket | `cold -> warming -> hot -> degraded` | 默认只保留 totals、sum、bucket；不得记录原始 trace | runtime / node 进程本地写，status 只读 |
| router timing snapshot | `prepare_subscriptions_ms_total`、`match_filters_ms_total`、`regex_compile_ms_total`、`prepared_hits`、`parse_fallbacks` | 准备订阅与匹配过滤器时更新累计指标 | `unprepared -> prepared -> matching` | 仅保留 bounded counters 与固定 bucket；按 stage 聚合 | 仅 router 路径写入 |
| status payload | `/v1/chain/status.wasm.build`、`/executor`、`/router`、`metrics_available`、`degraded_reason`、`observed_since_unix_ms` | 节点响应 status 请求时返回当前快照 | `available -> degraded -> unavailable` | 只暴露 bounded snapshot；不内嵌原始日志 | 节点读接口对观察者开放，写入限本地进程 |
| external summary | `window_ms`、`delta_calls`、`delta_compile_ms`、`delta_timeout_count`、`top_modules`、`p50/p95 buckets` | repo-owned 脚本读取 status samples 后输出 summary.md/json | `sampling -> aggregated -> archived` | 按窗口差分累计 counters；重启/reset 需缩窗 | 仅 operator/QA 脚本写 summary |
- Acceptance Criteria:
  - AC-1: `tools/wasm_build_suite` 必须为 canonical build 输出 `total_build_wall_ms` 与阶段耗时字段，且 dry-run 与真实构建在 schema 上可区分。
  - AC-2: `oasis7_wasm_executor` 必须区分 memory-cache hit、disk-cache hit、compile miss 三类路径，并为 `compile/deserialize/instantiate/entrypoint/decode` 提供累计耗时或固定 bucket。
  - AC-3: `oasis7_wasm_router` 必须输出 subscription prepare 与 filter match 的 timing/counter 指标，至少覆盖 prepared-hit、parse-fallback 与 regex-compile 三类信号。
  - AC-4: `/v1/chain/status` 必须新增 `wasm` section，并沿用 storage/traffic 的共享 snapshot 语义，不得把 timing 指标直接写入世界状态或共识数据。
  - AC-5: 默认 status payload 不得包含 `trace_id`、原始 input/output bytes、无界 `module_id -> metrics` map；若需要模块级明细，必须限制为 top-N 或显式 allowlist。
  - AC-6: 节点或 status lock 降级时，`degraded_reason` 必须显式说明 WASM metrics 不可用，但模块执行本身不得因此失败。
  - AC-7: repo 内必须提供可复用的 summary 入口，把 cumulative snapshot 转成窗口 delta、bucket-derived p50/p95 与热点摘要。
  - AC-8: 观测设计必须与现有 `TASK-WORLD_RUNTIME-060` perf probe、`chain-status-traffic-metrics` status snapshot 模式、`runtime-release-gate-metrics-template` 兼容，不得再引入一套平行口径。
- Non-Goals:
  - 不在本专题中定义新的 WASM ABI 或 host function 能力。
  - 不把 tracing span、open telemetry exporter 或远端 metrics backend 一并纳入首期范围。
  - 不在首期默认暴露逐 `trace_id` 或逐 payload 的超高基数明细。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `tools/wasm_build_suite`、`oasis7_wasm_executor`、`oasis7_wasm_router`、`oasis7_chain_runtime` status payload、repo-owned summary script。
- Evaluation Strategy: 以“是否能稳定区分 build/compile/cache/execute/router 热点”与“是否能在 status/summary 中输出 machine-readable 证据”为主，不以一次性 benchmark 截图代替正式验收。

## 4. Technical Specifications
- Architecture Overview: WASM observability 采用与 storage/traffic 一致的本地 cumulative snapshot 模型。build timing 进入 metadata/receipt；executor/router timing 在进程内聚合为共享 snapshot；`oasis7_chain_runtime` 负责通过 `/v1/chain/status.wasm` 暴露 bounded machine-readable 状态；外部脚本再将累计 counters 转换为窗口 summary。
- Integration Points:
  - `tools/wasm_build_suite/src/lib.rs`
  - `crates/oasis7_wasm_executor/src/lib.rs`
  - `crates/oasis7_wasm_router/src/lib.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/status_payload.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/storage_metrics.rs`
  - `doc/world-runtime/templates/runtime-release-gate-metrics-template.md`
  - `doc/world-runtime/project.md`
- Edge Cases & Error Handling:
  - metrics lock 中毒：必须返回 `degraded_reason`，但不得阻断正常 module call。
  - 节点重启 / `observed_since_unix_ms` 变化：外部 summary 必须自动缩窗，避免负 delta 或跨重启伪增量。
  - 冷启动 compile storm：允许 counters 快速增长，但 status payload 仍必须保持 bounded size。
  - 无模块执行流量：`metrics_available=false` 或 counters 保持 0，脚本不得伪造 p95。
  - 指标 schema 升级：必须保持 machine-readable 向后兼容或显式版本字段，避免 summary 脚本静默误读。
  - build dry-run：允许没有真实 timing，但必须显式标记而不是输出伪 `0ms success`。
  - top-N 裁剪：若模块级明细超过上限，必须保留 `truncated=true` 或等价信号，禁止静默丢弃且不告知。
- Non-Functional Requirements:
  - NFR-1: 默认 `/v1/chain/status.wasm` payload 必须保持 bounded cardinality，未启用模块级 top-N 时不得随模块总数线性增长。
  - NFR-2: 观测写入不得改变 deterministic execution 输出；所有 timing/metrics 仅限本地观测层，不得进入共识数据、world state 或 replay contract。
  - NFR-3: 默认状态下，WASM metrics instrumentation 对现有 release perf probe 的额外 wall-clock 开销目标不高于 `10%`。
  - NFR-4: status payload 在默认配置下必须可在单次请求中稳定序列化，建议预算 `<=64 KiB`；开启 bounded top-N 明细后建议预算 `<=128 KiB`。
  - NFR-5: 计时字段必须统一使用毫秒或微秒语义并显式标注单位，禁止不同子系统混用无单位整数。
  - NFR-6: 模块级明细若存在，默认上限不得超过 `top 10`，且不得包含原始 payload 内容。
- Security & Privacy: timing/metrics 输出不得暴露原始请求内容、私钥、auth proof 或其他敏感 payload；任何调试级明细都必须保持显式 env-gated，不进入默认 public status surface。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 定义 snapshot schema、status surface 与 build/executor/router 首期 timing 指标。
  - v1.1: 增加 repo-owned window summary、bucket-derived p50/p95 与 top-N hotspot 输出。
  - v2.0: 将 WASM observability 纳入 release gate / candidate evidence 模板，形成持续趋势对比。
- Technical Risks:
  - 风险-1: 过度细粒度埋点会把执行器热点从“不可观测”变成“观测本身过重”。
  - 风险-2: 若把 `module_id` 明细直接全量暴露到 status，节点侧 payload 会无界增长。
  - 风险-3: 若 build timing 与 executor timing 单位或采样口径不一致，后续 summary/incident 会出现错误归因。
  - 风险-4: 若 metrics 锁或 snapshot 刷新失败影响主执行路径，会把“性能观测”反向变成 runtime 可用性风险。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-036 | `wasm-observability-timing-metrics` | `test_tier_required` | 设计文档审查、`doc-governance-check`、状态字段/schema 校验用例、summary 脚本 dry-run | WASM build/executor/router 热点归因、节点 status 可观测性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-WMTM-001 | 复用 storage/traffic 的 cumulative snapshot -> status payload -> external summary 模式 | 直接把 perf probe 日志提升为正式指标来源 | 现有 status/summary 模式已被节点与 triad 流量观测验证，维护成本更低。 |
| DEC-WMTM-002 | 默认只暴露 bounded totals/buckets，模块级明细采用 top-N / allowlist | 默认输出全量 `module_id -> timing` map | 默认全量明细会造成 status payload 与 cardinality 无界增长。 |
| DEC-WMTM-003 | timing 指标只留在本地观测面，不进入 world state / replay contract | 把 timing 直接落入事件或状态供全链回放 | wall-clock timing 不具确定性，进入 world state 会破坏 replay contract。 |
