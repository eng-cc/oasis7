# oasis7 Runtime：WASM 执行器接入（设计分册）

- 对应设计文档: `doc/world-runtime/wasm/wasm-executor.design.md`
- 对应项目管理文档: `doc/world-runtime/wasm/wasm-executor.project.md`

审计轮次: 4


本分册描述将真实 WASM 执行器接入 `ModuleSandbox` 的最小方案。

## 1. Executive Summary
- 在现有 `ModuleSandbox` 抽象之上提供真实 WASM 执行实现（首选 Wasmtime）。
- 与既有 ABI/序列化约定对齐，保证输入/输出可验证、可回放。
- 提供确定性与资源限制（内存、燃料、超时、输出大小）并可审计。

## 2. User Experience & Functionality
### In Scope（V1）
- 以 `ModuleSandbox` 为适配层的执行器实现（不改动 world 内核调用流程）。
- 基本资源限制：内存上限、燃料/指令预算、超时、输出大小。
- 最小编译缓存：按 `wasm_hash` 缓存已编译模块，并可把 Wasmtime 序列化产物持久化到磁盘。
- 可配置的执行器参数（燃料、超时、并发上限、缓存容量）。
- 过渡期占位实现：未接入引擎时返回 `SandboxUnavailable`。
- 基础依赖通过 Cargo feature `wasmtime` 引入。
- 引擎骨架以 `Engine::default()` 初始化，后续再接入燃料/超时配置。

### Out of Scope（V1 不做）
- 多线程并行执行与跨模块共享状态。
- 复杂 I/O host functions（保持纯函数模型）。
- JIT 运行时热更新或远程分发。


## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（文档迁移任务）。
- Evaluation Strategy: 通过文档治理校验、引用扫描与任务日志检查验证迁移质量。

## 4. Technical Specifications
### 关键接口
- `ModuleSandbox`：保持现有 `call(request) -> ModuleOutput` 入口不变。
- `WasmExecutorConfig`（新增）：执行器配置（燃料、超时、内存上限、缓存上限）。
- `WasmExecutor`（新增实现）：封装底层引擎并实现 `ModuleSandbox`。

### 执行流程（概念）
1. 校验 `ModuleCallRequest`（limits 与运行时最大值）。
2. 按 `wasm_hash` 获取/编译模块（命中缓存或新编译）。
3. 绑定 host functions（仅暴露 ABI 必需的接口）。
4. 调用模块入口（`reduce` 或 `compute`），传入序列化输入。
5. 读取并反序列化输出，执行 `ModuleOutput` 校验。
6. 超时/超限返回 `ModuleCallFailure`，写入 `ModuleCallFailed` 事件。

### 资源限制与确定性
- **燃料/指令预算**：优先使用引擎原生 fuel/epoch 机制。
- **内存限制**：WASM memory pages + 运行时限制双重校验。
- **超时**：引擎 epoch 或外部 watchdog 触发超时。
- **确定性**：禁用非确定性 host function（时间、随机、I/O）。

### 实现要点（E2）
- Wasmtime 引擎启用 fuel + epoch interruption 以支持超时/燃料限制。
- 执行器在调用前预检查请求 limits（fuel/memory/output），并映射到 ModuleCallErrorCode。
- 输出校验失败路径单元测试覆盖 OutputTooLarge / Timeout 场景。

### 实现要点（E3）
- 编译缓存以 `wasm_hash` 为键，LRU 策略，容量由 `max_cache_entries` 控制。
- 缓存通过 `Arc<Mutex<...>>` 共享，允许多执行器克隆共享已编译模块。
- 编译过程与缓存锁分离，避免长时间持锁。
- 若配置 `compiled_cache_dir`，磁盘层必须持久化 Wasmtime `Module::serialize()` 产物，而不是原始 `.wasm` 字节；命中后优先走 `Module::deserialize_file()` 复用已编译工件，避免重启后再次 `Module::new(...)`。

### 实现要点（E4）
- Wasmtime 执行器使用 `memory`/`alloc`/`reduce|call` 导出进行最小调用（`reduce/call(i32, i32) -> (i32, i32)`，入口取决于 ModuleKind）。
- `ModuleCallRequest` 增加 `wasm_bytes`，由 `World::execute_module_call` 注入真实工件。
- 集成测试通过 `--features wasmtime` 验证真实 wasm 调用与回放事件一致性。

### 实现要点（E5）
- 输出采用 Canonical CBOR 解码为 `ModuleOutput`。
- 集成测试的 wasm 工件输出切换为 CBOR 编码。

### 实现要点（E6）
- 事件/动作输入改为 Canonical CBOR 编码，满足 `wasm-1` ABI 的确定性要求。
- 新增模块输入 CBOR 编码的路由测试。

### 实现要点（E7）
- 模块输入封装为 `ModuleCallInput { ctx, event|action }`，携带 `ModuleContext` 元信息。
- `ModuleContext` 包含 `v/module_id/trace_id/time/origin/limits` 等字段。
- 新增输入 envelope 编码测试，校验 ctx 与 event/action bytes。

### 实现要点（E8）
- `ModuleContext.world_config_hash` 使用当前 manifest 的哈希（`current_manifest_hash`）。
- 输入 envelope 测试校验 `world_config_hash` 一致性。

### 实现要点（E10）
- reducer 调用输入携带 `state`（空字节串代表无历史状态）。
- 模块返回 `new_state` 时记录 `ModuleStateUpdated` 并更新状态，保证回放一致。
- pure 模块返回 `new_state` 视为 InvalidOutput。

### 实现要点（E11）
- 将 `crates/oasis7/Cargo.toml` 的 `wasmtime` 依赖从 `18` 升级到 `41`，并刷新 `Cargo.lock`。
- 保持现有 `ModuleSandbox` 的 Wasmtime API 调用路径不变（`Config/Engine/Store/Linker/TypedFunc`），验证升级后可直接兼容。
- 升级后通过 `--features wasmtime` 执行 `cargo check` 与 `wasm_executor` 相关测试，确保执行器闭环可用。

### 实现要点（E12）
- `WasmExecutor` 初始化失败必须返回结构化错误，不允许以 `panic` 终止宿主。
- `oasis7_wasm_sdk::wire` 的 CBOR 编解码失败必须向调用者显式暴露，由 builtin 模块明确选择 fallback，而不是由 SDK 静默吞错。
- Node/runtime 入口在构造执行器失败时需保留可观测错误文本，测试需覆盖磁盘缓存初始化失败路径。

## 5. Risks & Roadmap
- **E1**：选择 WASM 引擎并完成配置结构体与沙箱实现骨架。
- **E2**：接入燃料/超时/内存限制，输出校验与错误码映射。
- **E3**：实现编译缓存与并发安全策略。
- **E4**：补充集成测试（真实 wasm、超限失败、确定性回放）。
- **E5**：切换 ModuleOutput 编码为 Canonical CBOR，并完善 ABI 说明与测试。
- **E6**：模块输入切换为 Canonical CBOR 编码并补充测试。
- **E7**：模块输入封装 ModuleContext + event/action envelope 并补充测试。
- **E8**：补充 world_config_hash 并测试。
- **E9**：模块调用入口按 ModuleKind 选择并补充测试。
- **E10**：模块状态输入/更新接入并补齐回放一致性测试。
- **E11**：升级 Wasmtime 版本（18 -> 41）并完成兼容性回归验证。
- **E12**：清理执行器初始化 `panic` 与 SDK wire 静默吞错路径，补足失败路径结构化错误回归。
- **E13**：将磁盘编译缓存从“原始 wasm 回盘”修正为“序列化 compiled artifact 回盘”，补齐 round-trip 与损坏恢复回归。

### Technical Risks
- 引擎版本升级导致行为变化（需锁定版本/回放验证）。
- 资源限制不一致（引擎与内核限制口径差异）。
- ABI 变更导致兼容性破坏（需版本化接口）。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-006 | 文档内既有任务条目 | `test_tier_required` | `./scripts/doc-governance-check.sh` + 引用可达性扫描 | 迁移文档命名一致性与可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-DOC-MIG-20260303 | 逐篇阅读后人工重写为 `.prd` 命名 | 仅批量重命名 | 保证语义保真与审计可追溯。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章 Executive Summary。
- 原“范围” -> 第 2 章 User Experience & Functionality。
- 原“接口 / 数据” -> 第 4 章 Technical Specifications。
- 原“里程碑/风险” -> 第 5 章 Risks & Roadmap。
