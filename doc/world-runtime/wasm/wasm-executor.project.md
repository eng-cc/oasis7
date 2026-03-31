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

## 依赖
- doc/world-runtime/wasm/wasm-executor.prd.md
- `ModuleSandbox` 接口与模块 ABI 文档（`doc/world-runtime/wasm/wasm-interface.md`）
- 模块加载缓存与存储实现（`doc/world-runtime/module/module-storage.prd.md`）

## 状态
- 当前阶段：E12（初始化失败结构化错误与 SDK wire 显式错误闭环完成）
