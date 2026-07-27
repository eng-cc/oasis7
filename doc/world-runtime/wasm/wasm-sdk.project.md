# WASM SDK 兼容与 Wire 项目追踪

- 对应需求文档：`doc/world-runtime/wasm/wasm-sdk.prd.md`
- 对应设计文档：`doc/world-runtime/wasm/wasm-sdk.design.md`
- 当前状态：no_std 与 wire 类型收敛已完成并归档到稳定专业权威。

## 任务拆解

- [x] wasm-sdk-no-std (PRD-WORLD_RUNTIME-042) [test_tier_required]: SDK 默认 no_std、显式 `std` feature，并保持生命周期与导出 ABI。 Trace: #2668 (task_dcb5171aaffd48f8bead02c326045d5e)
  - 历史来源：NSDK-1/2。
- [x] wasm-sdk-wire-types (PRD-WORLD_RUNTIME-042) [test_tier_required]: SDK 持有共享 wire 类型/helper，builtin 模块移除重复协议定义。 Trace: #2668 (task_dcb5171aaffd48f8bead02c326045d5e)
  - 历史来源：WIRESDK-1/2。
- [x] wasm-sdk-authority-consolidation (PRD-WORLD_RUNTIME-042) [test_tier_required]: 吸收两组完成专题、修正 codec 失败语义并删除六个已吸收源文件。 Trace: #2668 (task_dcb5171aaffd48f8bead02c326045d5e)

## 依赖

- Runtime ABI reference：`doc/world-runtime/wasm/wasm-interface.md`
- Executor/manifest authority：`doc/world-runtime/wasm/wasm-executor.prd.md`
- SDK 与 builtin 实现：`crates/oasis7_wasm_sdk`、`crates/oasis7_builtin_wasm_modules/*`

## Evidence ledger

| 能力 | 当前实现/验证入口 |
| --- | --- |
| 默认 no_std 与显式 std | `crates/oasis7_wasm_sdk/src/lib.rs`、SDK Cargo features、SDK unit tests |
| 生命周期与导出 ABI | `LifecycleStage`、`WasmModuleLifecycle`、`dispatch_reduce`、`dispatch_call`、`export_wasm_module!` |
| 共享 wire schema | `oasis7_wasm_sdk::wire` 的 input/context/effect/emit/output 与 CBOR helpers |
| Builtin 复用 | `crates/oasis7_builtin_wasm_modules/*`、m1/m4 sync/check 与 required-tier build |
| 错误可观测 | wire helper 的结构化错误及 builtin 调用点的显式 fallback |

## 维护与验证

- `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_sdk`
- wasm32 target 可用时运行 SDK wasm32 check；不可用时记录 skipped。
- 扫描 builtin 模块中的重复 `ModuleCallInput/ModuleContext/ModuleOutput` 定义。
- 运行代表性 builtin sync/check 与 repository required gate。
- `./scripts/doc-governance-check.sh`
- `./scripts/readme-link-check.sh`
- `git diff --check`

## 边界

- 本状态不等于所有第三方模块、所有 target、集成、长稳或发布就绪。
- ABI/manifest/hash、sandbox 与 storage 的权威分别留在对应 WASM/module 文档。

## 状态

- 当前状态：no_std 与 wire 类型收敛已完成并归档到稳定专业权威。
- 阻塞项：无。
- 非声明：不等于所有 target、第三方模块、集成、长稳或发布就绪。
