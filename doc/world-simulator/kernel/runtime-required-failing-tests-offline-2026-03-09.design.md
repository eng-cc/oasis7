# runtime required 失败用例临时下线设计（2026-03-09）

- 对应需求文档: `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.prd.md`
- 对应项目管理文档: `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.project.md`

## 1. 设计定位
定义 runtime required 测试链路的最小止血方案：仅对白名单内 10 个已知失败用例加 `#[ignore]`，恢复 required 套件可执行，同时保留失败签名与恢复锚点。

## 2. 设计结构
- 白名单控制层：固定 10 个测试名，禁止模块级或通配式扩大忽略范围。
- 注解收敛层：在测试函数级增加 `#[ignore]` 与失败原因说明，保留原实现与断言。
- 验证链路层：required 测试继续执行非白名单项，并以 ignored 数量与失败列表校验变更边界。
- 恢复治理层：当 `m1` builtin wasm 的 hash manifest、identity manifest 与 DistFS blobs 对齐后，逐项移除 ignore 并恢复定向 wasmtime 回归。

## 3. 关键接口 / 入口
- `crates/oasis7/src/runtime/tests/agent_default_modules.rs`
- `crates/oasis7/src/runtime/tests/power_bootstrap.rs`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required`

## 4. 约束与边界
- ignore 数量必须精确等于 10，不能新增隐藏白名单。
- 下线原因必须在代码中可追溯，禁止无说明 ignore。
- 生产代码零改动，变更范围限定在测试层和文档台账。
- 本专题不修复根因，只恢复 required 链路可执行性。

## 5. 设计演进计划
- 先冻结白名单与失败签名。
- 再在函数级精确下线并重新跑 required 套件。
- 最后在 `m1` builtin manifest/hash/DistFS 修复后逐项回收 ignore，并执行定向 wasmtime 回归证明恢复。
