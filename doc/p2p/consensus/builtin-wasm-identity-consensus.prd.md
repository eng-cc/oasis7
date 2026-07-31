# P2P Builtin Wasm 身份共识与跨平台构建方案

- 对应设计文档: `doc/p2p/consensus/builtin-wasm-identity-consensus.design.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 解决“不同宿主构建产物 hash 漂移导致门禁不稳定”的生产问题，同时保留节点本地可构建能力。
- Proposed Solution: 将一致性目标从“跨平台产物字节 hash 必须一致”升级为“跨平台模块身份（identity）一致”。
- Success Criteria:
  - SC-1: 在不要求 runtime 节点执行前本机重编源码的前提下，支持多节点验证、产物复用与 identity 一致性校验。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want P2P Builtin Wasm 身份共识与跨平台构建方案 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: 为 builtin wasm 增加 identity manifest（`source_hash`、`build_manifest_hash`、`identity_hash`）。
  - AC-2: 扩展 `sync-m1/m4/m5` 流程：在同步 hash 清单时同时生成/校验 identity manifest。
  - AC-3: 运行时读取 identity manifest，为 bootstrap module manifest 写入可验证的 `artifact_identity`。
  - AC-4: 校验策略升级：`sync --check` 同时验证当前平台产物 hash 与 identity 数据完整性。
  - AC-5: 统一 required/pre-commit 门禁到 `m1/m4/m5` 三套 builtin 模块，避免只校验 `m1` 造成“部分模块漂移漏检”。
  - AC-6: runtime builtin materializer 支持非 canonical 平台节点的本地回退编译与缓存复用，不再把“hash 必须命中清单”作为唯一可用条件。
- Non-Goals:
  - 修改 wasm ABI 协议版本与 runtime 模块执行语义。
  - 将 identity 共识专题扩展为发布级 Docker builder 设计；当前 canonical build 目标态由 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md` 管理。
  - 替换现有 `sha256` 算法。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md`
  - GitHub Issue / GitHub Project
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
- 现有 hash 清单继续保留：
  - `crates/oasis7/src/runtime/world/artifacts/m1_builtin_modules.sha256`
  - `crates/oasis7/src/runtime/world/artifacts/m4_builtin_modules.sha256`
  - `crates/oasis7/src/runtime/world/artifacts/m5_builtin_modules.sha256`
- 新增 identity manifest：
  - `crates/oasis7/src/runtime/world/artifacts/m1_builtin_modules.identity.json`
  - `crates/oasis7/src/runtime/world/artifacts/m4_builtin_modules.identity.json`
  - `crates/oasis7/src/runtime/world/artifacts/m5_builtin_modules.identity.json`
- runtime 本地缓存索引（新增）：
  - ` .distfs/builtin_wasm/module_hash_index.txt`
  - 语义：记录 `module_id -> 最近一次可验证加载成功的 wasm_hash`，用于非 canonical 平台复用本地构建产物，避免每次启动重复编译。
- identity manifest 每个模块至少包含：
  - `module_id`
  - `source_hash`：模块源码输入集摘要（基于模块 crate 目录 + Cargo.lock）
  - `build_manifest_hash`：构建配方摘要（toolchain/target/build-std/canonicalizer）
  - `identity_hash`：`sha256("<module_id>:<source_hash>:<build_manifest_hash>")`
- 运行时接口：
  - `runtime/m{1,4,5}_builtin_wasm_artifact.rs` 增加读取 identity manifest 的辅助函数。
  - `runtime/world/bootstrap_{power,economy,gameplay}.rs` 改为使用 identity manifest 生成 `ModuleArtifactIdentity`。
  - `runtime/builtin_wasm_materializer.rs` 在“远程拉取失败 + 本地编译回退”路径允许落地非清单 hash（仅本地编译路径），并写入模块 hash 索引。
- 门禁接口：
  - `scripts/ci-tests.sh required` 统一执行：
    - `./scripts/sync-m1-builtin-wasm-artifacts.sh --check`
    - `./scripts/sync-m4-builtin-wasm-artifacts.sh --check`
    - `./scripts/sync-m5-builtin-wasm-artifacts.sh --check`

## 5. Risks & Roadmap
- Phased Rollout:
  - M1：设计文档、项目管理文档落地。
  - M2：`sync-m1` 主脚本扩展 identity manifest 生成与校验，并对 `m4/m5` 复用。
  - M3：runtime 接入 identity manifest，替换 bootstrap 占位 identity 逻辑。
  - M4：测试补齐与 required 回归通过。
  - M5：项目文档与 devlog 收口。
  - M6：全系统迁移（CI/pre-commit/runtime fallback）与过时文档归档完成。
- Technical Risks:
  - `source_hash` 输入集合定义不稳定会导致无意义抖动，需要固定“仅模块 crate + Cargo.lock”。
  - 旧版本清单迁移窗口内可能出现“有 hash 无 identity”状态，需提供兼容提示与一次性同步脚本。
  - 若开发者在非标准环境覆盖构建参数，`build_manifest_hash` 会变化并触发门禁，需要清晰错误提示。
  - 非 canonical 平台允许本地回退编译后，必须保证远程拉取路径仍然严格校验 hash 清单，避免把“跨平台兼容”退化成“任意字节可执行”。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-057-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-057-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
