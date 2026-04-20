# oasis7 Runtime：WASM 模块标准化观测与契约测试（项目管理文档）

- 对应需求文档: `doc/world-runtime/wasm/wasm-module-observability-standardization.prd.md`
- 对应设计文档: `doc/world-runtime/wasm/wasm-module-observability-standardization.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] wasm-module-observability-standardization (PRD-WORLD_RUNTIME-037) [test_tier_required]: 落地 module-local observe spec、通用 `wasm_module_observe` runner、repo-owned wrapper script、模板与 `m1_rule_move` 代表样例，并回写 `world-runtime` 根 PRD / project / README / prd.index。 Trace: .pm/tasks/task_20b6ee42182247ccbebe6a6a2c2db469.yaml
  - 产物文件:
    - `tools/wasm_module_observe/Cargo.toml`
    - `tools/wasm_module_observe/src/lib.rs`
    - `tools/wasm_module_observe/src/main.rs`
    - `tools/wasm_module_observe/src/spec.rs`
    - `tools/wasm_module_observe/src/report.rs`
    - `tools/wasm_module_observe/tests/m1_rule_move.rs`
    - `scripts/oasis7-wasm-module-observe.sh`
    - `crates/oasis7_builtin_wasm_modules/_templates/module_observe.json`
    - `crates/oasis7_builtin_wasm_modules/m1_rule_move/observability/module_observe.json`
    - `doc/world-runtime/prd.md`
    - `doc/world-runtime/project.md`
    - `doc/world-runtime/README.md`
    - `doc/world-runtime/prd.index.md`
    - `doc/world-runtime/wasm/wasm-module-observability-standardization.prd.md`
    - `doc/world-runtime/wasm/wasm-module-observability-standardization.design.md`
    - `doc/world-runtime/wasm/wasm-module-observability-standardization.project.md`
    - `.pm/tasks/task_20b6ee42182247ccbebe6a6a2c2db469.execution.md`
  - 验收命令 (`test_tier_required`):
    - `env -u RUSTC_WRAPPER cargo test --manifest-path tools/wasm_module_observe/Cargo.toml --offline`
    - `env -u RUSTC_WRAPPER cargo run --manifest-path tools/wasm_module_observe/Cargo.toml -- observe --spec crates/oasis7_builtin_wasm_modules/m1_rule_move/observability/module_observe.json --out-dir .tmp/wasm_module_observe_m1_check`
    - `bash -n scripts/oasis7-wasm-module-observe.sh`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`

## 依赖
- `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- `tools/wasm_build_suite/src/lib.rs`
- `crates/oasis7_wasm_executor/src/lib.rs`
- `crates/oasis7_wasm_router/src/lib.rs`

## 状态
- 更新日期: 2026-04-20
- 当前阶段: MVP 已落地，runner 与代表模块样例已可执行
- owner role: `wasm_platform_engineer`
- 联审角色: `runtime_engineer`、`qa_engineer`
- 当前阻塞:
  - 首期只补了 `m1_rule_move` 代表样例，更多 builtin 模块仍需逐步补 module-local spec。
- 实施备注:
  - runner 只接受 spec 驱动，不接受模块特例硬编码。
  - summary 默认保留 bounded 输出，不导出逐 trace 原始 payload 全量转储。
