# WASM SDK 兼容与 Wire 契约

- 对应设计文档：`doc/world-runtime/wasm/wasm-sdk.design.md`
- 稳定证据入口：`doc/world-runtime/wasm/evidence.md`
- 专业权威：`wasm_platform_engineer`

## 1. 目标

为 builtin 与第三方 WASM 模块提供单一、轻量、可移植的 SDK 契约。SDK 必须默认适配 `no_std` 模块环境，并以唯一的 Canonical-CBOR wire 类型避免模块间协议漂移，同时保持既有 `wasm-1` ABI 和模块行为兼容。

## 2. 范围

### 永久契约

1. `oasis7_wasm_sdk` 默认在非 test、未启用 `std` feature 时使用 `no_std`，需要动态分配的能力来自 `alloc`；宿主或测试只有显式启用 `std` 才可依赖标准库。
2. `alloc`、`reduce`、`call` 导出，`LifecycleStage`、`WasmModuleLifecycle`、`dispatch_reduce`、`dispatch_call` 与 `export_wasm_module!` 保持稳定；SDK 内部收敛不得静默改变模块种类、生命周期或业务效果。
3. `ModuleCallInput`、`ModuleContext`、`ModuleEffectIntent`、`ModuleEmit`、`ModuleOutput` 及其编码 helper 由 SDK 持有一份 wire 定义。builtin 模块只保留领域结构，不得复制一套并行协议。
4. wire 编解码使用与 runtime `wasm-1` 一致的 Canonical CBOR 字段和默认值；输入、输出与 `output_bytes` 口径必须保持兼容。
5. codec 失败必须以显式 `Result` 或调用点明确选择的 fallback 处理；SDK 不得把损坏输入静默转换为空输出或 `None`。
6. 新字段或 feature 必须说明默认值、旧模块兼容性和 wasm32 构建影响。破坏性 ABI 变化不能通过 SDK 整理任务夹带进入。

## 3. 接口 / 数据

- 权威 crate：`crates/oasis7_wasm_sdk`
- wire feature：承载共享输入、上下文、effects、emits、输出与 helper。
- builtin 使用者：`crates/oasis7_builtin_wasm_modules/*`
- runtime ABI 与执行语义继续由 `wasm-interface.md`、`wasm-executor.prd.md` 和 `oasis7_wasm_abi` 拥有。
- 本文不拥有模块业务规则、hash/manifest 发布链、执行器资源限制或模块存储生命周期。

## 4. 验收

- SDK 单测覆盖生命周期 dispatch、默认分配和 wire round-trip / 失败路径。
- 在 wasm32 target 可用时验证默认 no_std 编译；target 不可用必须记录为 skipped，不能报告为通过。
- builtin 模块迁移需证明不再保留重复 wire 定义，并通过代表性 sync/check 与 required-tier 编译。
- SDK 变更必须明确向后兼容或提供迁移路径。

## 5. 风险与非声明

- `std` feature 或 serde feature 组合可能重新引入隐式宿主依赖。
- 批量复用共享 wire 类型可能暴露历史字段差异，必须逐项校验，不能仅依靠文本替换。
- 本文不宣称所有第三方模块、所有 target 或发布候选已验证。

## 6. Validation & Decision Record

SDK ABI、wire 使用和 wasm32 兼容的稳定验证入口见
[`evidence.md`](evidence.md)。任务批次、当前状态和已完成工作历史由 GitHub
task issue / Project 与 Git history 承接。

| PRD-ID | 测试层级 | 验证方法 | 回归范围 |
| --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-042 | `test_tier_required` | SDK tests、可用时 wasm32 check、builtin wire usage scan、required-tier build | SDK ABI、builtin 模块编译与 wire 兼容 |

| 决策 ID | 选定方案 | 否决方案 | 依据 |
| --- | --- | --- | --- |
| DEC-WR-WASM-SDK-001 | 以稳定 `wasm-sdk` 权威合并 no_std 与 wire-dedup 完成记录 | 长期保留两组三件套作为并列权威 | 两者共同约束同一 SDK 兼容面；单一权威更能防止 feature、wire 和错误语义漂移。 |
| DEC-WR-WASM-SDK-002 | codec 失败显式返回或由调用点选择 fallback | SDK 静默返回空输出/`None` | 当前结构化错误契约要求失败可观测，旧静默措辞已失效。 |
