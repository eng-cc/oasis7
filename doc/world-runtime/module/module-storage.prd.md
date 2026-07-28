# oasis7 Runtime：模块存储持久化（设计文档）

- 对应设计文档: `doc/world-runtime/module/module-storage.design.md`
- 对应项目管理文档: `doc/world-runtime/module/module-storage.project.md`

审计轮次: 4


## 1. Executive Summary
- 提供模块工件（WASM bytes）与模块注册表（registry）的**可持久化存储**。
- 支持冷启动加载模块注册表与工件元信息，保证回放与治理闭环一致性。
- 提供可测试、可替换的本地文件存储实现（未来可替换为 KV/对象存储）。

## 2. User Experience & Functionality
### In Scope
- 本地目录结构与文件格式约定（registry/meta/artifacts）。
- `ModuleStore` 读写 API（写入工件、读写注册表、读写 meta）。
- 版本号校验与基础错误处理。
- `World` 侧集成 API：保存/加载模块存储（S2）。

### Out of Scope
- 远端对象存储、分布式一致性、加密存储。
- 大规模分片、增量同步与压缩。


## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（文档迁移任务）。
- Evaluation Strategy: 通过文档治理校验、引用扫描与任务日志检查验证迁移质量。

## 4. Technical Specifications
### 存储布局（默认）
- `module_registry.json`：模块索引与活动版本
- `modules/<wasm_hash>.wasm`：模块工件（只读）
- `modules/<wasm_hash>.meta.json`：模块元信息（ModuleManifest 快照）

### 关键数据结构（示意）
```rust
struct ModuleRegistryFile {
  version: u64,
  updated_at: i64,
  records: BTreeMap<String, ModuleRecord>,
  active: BTreeMap<String, String>,
}
```

### ModuleStore API（示意）
- `ModuleStore::new(root)`：基于根目录构造存储
- `write_artifact(wasm_hash, bytes)`：写入工件
- `read_artifact(wasm_hash)`：读取工件
- `write_meta(manifest)`：写入 meta.json
- `read_meta(wasm_hash)`：读取 meta.json
- `save_registry(registry)`：保存注册表
- `load_registry()`：读取注册表并校验版本

### World 集成 API（示意）
- `World::save_module_store_to_dir(dir)`：保存 registry/meta/artifacts
- `World::load_module_store_from_dir(dir)`：加载 registry/meta/artifacts
- `World::save_to_dir_with_modules(dir)`：保存 world + 模块存储（同目录）
- `World::load_from_dir_with_modules(dir)`：加载 world + 模块存储（同目录）
  - 读取 registry
  - 校验 meta 与 registry 一致
  - 加载工件 bytes 到内存缓存

### 版本策略
- `module_registry.json` 使用 `version=1`，不支持版本直接拒绝加载。

## 5. Risks & Roadmap
- **S1**：实现本地文件存储 ModuleStore
- **S2**：接入 world 保存/加载流程

### Technical Risks
- 文件写入中断导致部分写入，需要原子写入策略。
- 工件体积大，读写影响性能。
- 版本升级时兼容性处理复杂。

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

## 7. 默认持久化与实例恢复合同

- `save_to_dir` / `load_from_dir` 的默认闭环包含 module registry、manifest metadata 与内容寻址 artifact；显式 `*_with_modules` 入口只保留兼容/定向调用价值，不得形成第二套持久化真值。
- 旧 snapshot 或目录没有 module store 时按兼容默认加载；一旦存在 registry/meta/artifact，恢复必须校验三者一致性与 artifact hash，损坏或缺失以结构化错误或受治理恢复处理，不静默替换。
- 持久实例至少保持 module identity/version/hash、owner、install target、active state 与安装时间的可回放关系。恢复后按稳定 instance identity 路由，不能回退到同 `module_id` 的全局覆盖语义。
- 升级后的 registry、instance state 与事件链必须原子一致；接口或订阅不兼容、owner 不匹配或 artifact 校验失败时不写入部分升级结果。
