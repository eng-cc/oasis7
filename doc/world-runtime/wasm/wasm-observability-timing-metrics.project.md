# oasis7 Runtime：WASM 可观测性与耗时指标（项目管理文档）

- 对应设计文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.design.md`
- 对应需求文档: `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] wasm-observability-timing-metrics-doc-surface (PRD-WORLD_RUNTIME-036) [test_tier_required]: 建立 `WASM 可观测性与耗时指标` 专题三件套，并回写 `world-runtime` 根 PRD / project / README / prd.index。 Trace: .pm/tasks/task_f0830d708c3b4f7abeea8cecf73053e4.yaml
  - 产物文件:
    - `doc/world-runtime/prd.md`
    - `doc/world-runtime/project.md`
    - `doc/world-runtime/README.md`
    - `doc/world-runtime/prd.index.md`
    - `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
    - `doc/world-runtime/wasm/wasm-observability-timing-metrics.design.md`
    - `doc/world-runtime/wasm/wasm-observability-timing-metrics.project.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "PRD-WORLD_RUNTIME-036|wasm-observability-timing-metrics|/v1/chain/status.wasm" doc/world-runtime/prd.md doc/world-runtime/project.md doc/world-runtime/README.md doc/world-runtime/prd.index.md doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md doc/world-runtime/wasm/wasm-observability-timing-metrics.design.md doc/world-runtime/wasm/wasm-observability-timing-metrics.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] wasm-observability-timing-metrics-mvp-implementation (PRD-WORLD_RUNTIME-036) [test_tier_required]: 落地首期 `build timing + executor/router cumulative snapshot + /v1/chain/status.wasm` 实现，并补最小 repo-owned summary 入口。 Trace: .pm/tasks/task_90d0ee7aa1464f248f717ff600e22b21.yaml
  - 产物文件:
    - `tools/wasm_build_suite/src/lib.rs`
    - `crates/oasis7_wasm_executor/src/lib.rs`
    - `crates/oasis7_wasm_executor/src/metrics.rs`
    - `crates/oasis7_wasm_router/src/lib.rs`
    - `crates/oasis7_wasm_router/src/metrics.rs`
    - `crates/oasis7/src/bin/oasis7_chain_runtime/status_payload.rs`
    - `crates/oasis7/src/bin/oasis7_chain_runtime/oasis7_chain_runtime_observability_tests.rs`
    - `scripts/oasis7-node-wasm-metrics-monitor.sh`
    - `.pm/tasks/task_90d0ee7aa1464f248f717ff600e22b21.yaml`
    - `.pm/tasks/task_90d0ee7aa1464f248f717ff600e22b21.execution.md`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test --manifest-path tools/wasm_build_suite/Cargo.toml -- --nocapture`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_executor -p oasis7_wasm_router`
    - `env -u RUSTC_WRAPPER cargo test -p oasis7 oasis7_chain_runtime_observability -- --nocapture`
    - `bash -n scripts/oasis7-node-wasm-metrics-monitor.sh`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`

## 后续实现切片建议
- WMTM-1: 为 `tools/wasm_build_suite` 增加 canonical build timing schema，并将 `total_build_wall_ms/cargo_build_ms/canonicalize_ms/hash_ms/receipt_write_ms` 写入 metadata/receipt。
- WMTM-2: 为 `oasis7_wasm_executor` 建立共享 cumulative snapshot，覆盖 cache hit/miss、compile/deserialize/instantiate/call/decode timing 与 `failure_by_code`。
- WMTM-3: 在 `oasis7_chain_runtime` 的 `/v1/chain/status` 中新增 `wasm` section，并以 storage/traffic 同款 snapshot 语义暴露 executor/router/build 指标。
- WMTM-4: 新增 repo-owned wasm metrics summary 入口，将 cumulative snapshot 转为窗口 delta、bucket-derived p50/p95 与热点摘要。
- WMTM-5: 为 `oasis7_wasm_router` 增加 prepare/match timing 指标与 bounded top-N 策略，并补齐 cardinality / payload budget 回归。

## 依赖
- `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- `doc/world-runtime/wasm/wasm-executor.prd.md`
- `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- `tools/wasm_build_suite/src/lib.rs`
- `crates/oasis7_wasm_executor/src/lib.rs`
- `crates/oasis7_wasm_router/src/lib.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/status_payload.rs`
- `doc/world-runtime/templates/runtime-release-gate-metrics-template.md`

## 状态
- 更新日期: 2026-04-20
- 当前阶段: 文档建模与首期 MVP 实现已完成；当前已落地 build timing、executor/router cumulative snapshot、`/v1/chain/status.wasm` 与最小 summary 入口，后续增量聚焦窗口 delta / bucket-derived p50/p95 / hotspot summary
- owner role: `wasm_platform_engineer`
- 联审角色: `runtime_engineer`、`producer_system_designer`
- 验证角色: `qa_engineer`
- 当前阻塞:
  - `status.wasm.build` 首期已通过显式 metadata/receipt 路径与顶层 degraded reason 暴露“部分可用”边界，但 build suite timing 与 runtime live snapshot 仍分属不同进程，后续窗口汇总脚本需要继续避免把非同一进程样本误判为同窗真值。
  - repo-owned summary 入口当前仍是 latest snapshot 汇总，尚未输出 reset-aware window delta、bucket-derived p50/p95 与 hotspot ranking。
- 实施备注:
  - 首期坚持 bounded snapshot，不默认暴露无界 `module_id -> timing` 明细。
  - timing 指标只留在本地观测面，不进入共识数据、world state 或 replay contract。
