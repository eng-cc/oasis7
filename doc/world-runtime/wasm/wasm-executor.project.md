# oasis7 Runtime：WASM 执行器接入（项目管理文档）

- 对应设计文档: `doc/world-runtime/wasm/wasm-executor.design.md`
- 对应需求文档: `doc/world-runtime/wasm/wasm-executor.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
- [x] E1 定义执行器配置结构（WasmExecutorConfig）
- [x] E1 实现 `ModuleSandbox` 的执行器骨架（占位实现）
- [x] E1 选择 WASM 引擎并落地基础依赖（Wasmtime + feature）
- [x] E1 初始化 Wasmtime 引擎骨架（Engine::default 占位）
- [x] E2 接入燃料/超时/内存限制与错误码映射
- [x] E2 补充输出校验失败路径测试（超限/超时）
- [x] E3 编译缓存与并发安全策略
- [x] E4 集成测试：真实 wasm 调用、确定性回放
- [x] 文档更新：运行时集成分册补充执行器细节
- [x] E5 切换 ModuleOutput ABI 为 Canonical CBOR
- [x] E5 引入 CBOR 编解码与输出校验测试
- [x] E5 更新 wasm-interface 与执行器文档说明
- [x] E6 模块输入采用 Canonical CBOR 编码
- [x] E6 补充模块输入 CBOR 编码测试
- [x] E7 模块输入封装 ModuleContext + event/action envelope
- [x] E7 补充输入 envelope 编码测试
- [x] E8 补充 world_config_hash（manifest hash）到 ModuleContext
- [x] E8 补充 world_config_hash 测试
- [x] E9 模块调用入口按 ModuleKind 选择（reduce/call）
- [x] E9 补充入口选择测试
- [x] E10 reducer 输入携带 state（空字节串默认）
- [x] E10 new_state 触发 ModuleStateUpdated 并回放一致
- [x] E10 补充模块状态输入/更新测试
- [x] E10 pure 模块返回 new_state 视为 InvalidOutput
- [x] E10 模块状态回放/快照一致性测试
- [x] E11 升级 Wasmtime 依赖版本（18 -> 41）并刷新锁文件
- [x] E11 执行 `--features wasmtime` 编译与执行器回归测试
- [x] E12 执行器初始化错误结构化返回，移除 `panic` 路径
- [x] E12 `oasis7_wasm_sdk::wire` 改为显式暴露 CBOR 解码失败，builtin 模块调用点改为显式 fallback
- [x] E12 补充磁盘缓存初始化失败与执行器调用点回归
- [x] wasm-executor-real-compiled-cache (PRD-WORLD_RUNTIME-002) [test_tier_required]: 将磁盘 compiled cache 从“原始 wasm 字节回盘”修正为“Wasmtime 序列化 compiled artifact 回盘”，并补齐 round-trip / 损坏恢复 / perf probe 测试可编译性。 Trace: .pm/tasks/task_c7a8defc7c0f4f4c8f86660b50df08a5.yaml
- [x] wasm-executor-agent-os-alignment (PRD-WORLD_RUNTIME-002) [test_tier_required]: 以可选兼容字段落地 ABI/schema、cap slot、pure policy hook、ModuleContext 与 compiled cache。 Trace: #2668 (task_dcb5171aaffd48f8bead02c326045d5e)
  - 历史来源：AOSA-1/2/3/5/6。
- [x] wasm-executor-sandbox-hardening (PRD-WORLD_RUNTIME-002) [test_tier_required]: 落地 fuel fallback、epoch 抢占、memory limiter、工件 hash 校验和结构化失败。 Trace: #2668 (task_dcb5171aaffd48f8bead02c326045d5e)
  - 历史来源：T1/T2/T3。
- [x] wasm-executor-authority-consolidation (PRD-WORLD_RUNTIME-002) [test_tier_required]: 吸收两组完成专题并删除六个重复源文件。 Trace: #2668 (task_dcb5171aaffd48f8bead02c326045d5e)

## 依赖
- doc/world-runtime/wasm/wasm-executor.prd.md
- `ModuleSandbox` 接口与模块 ABI 文档（`doc/world-runtime/wasm/wasm-interface.md`）
- 模块加载缓存与存储实现（`doc/world-runtime/prd.md`）

## 补充 evidence ledger

| 契约 | 当前实现/验证入口 |
| --- | --- |
| ABI/schema 与 capability | `crates/oasis7_wasm_abi/src/lib.rs`、`crates/oasis7/src/runtime/world/module_runtime.rs` 的 optional contract、cap slot/conflict 与 policy-hook tests |
| ModuleContext | runtime envelope 编码及 manifest/context 默认兼容 tests |
| Fuel/epoch/memory | `crates/oasis7_wasm_executor/src/lib.rs` 的 zero-gas fallback、interrupt/out-of-fuel 与 memory-growth tests |
| Artifact integrity | module persistence SHA-256 mismatch rejection tests |
| Compiled cache | serialized artifact round-trip、wrapper/precompiled marker、engine/OS/arch partition 与 corruption-as-miss tests |
| Structured failure | executor initialization/cache-dir/codec failure tests；无 panic 宿主退出 |

## 状态
- 当前阶段：E14（executor/security/alignment 稳定专业权威合并完成）；该状态不等于集成、长稳或发布就绪。
