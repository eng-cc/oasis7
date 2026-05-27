# Non-Viewer 设计一致性审查 Round2（2026-02-25）

- 对应设计文档: `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.design.md`
- 对应项目管理文档: `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.project.md`

审计轮次: 4


## 1. Executive Summary
- 在已完成 `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md` 的基础上，继续执行第二轮 non-viewer 设计一致性审查。
- 覆盖非 Viewer 的活跃设计分册（优先 `testing` / `p2p` / `world-runtime`），识别“设计要求与实现行为不一致”项。
- 对确认问题统一收敛优化，并同步文档与任务日志。

## 2. User Experience & Functionality
### In Scope
- 新增并维护第二轮审查记录：问题台账、核对范围、结论。
- 记录并跟踪已发现问题：non-viewer Rust 单文件超过 1200 行的工程约束偏差。
- 核对非 Viewer 代码实现与设计文档的一致性（含必要定向测试）。
- 对本轮确认问题执行一并优化（代码/文档）。

### Out of Scope
- `crates/oasis7_viewer` 及 Viewer UI/Web 交互代码。
- 仅归档文档的历史一致性回溯重审。


## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（文档迁移任务）。
- Evaluation Strategy: 通过文档治理校验、引用扫描与任务日志检查验证迁移质量。

## 4. Technical Specifications
- 问题台账字段：`id` / `scope` / `design_source` / `implementation` / `status` / `notes`。
- 约束项：
  - Rust 文件行数上限：1200（以仓库工作流约束为准）。
  - 设计一致性判定：文档声明行为与运行时实际行为/测试结果一致。
- 输出物：
  - 本设计文档与项目管理文档状态更新。
  - 任务日志（`doc/devlog/README.md`）。

## 5. Risks & Roadmap
- M0：建档并记录已发现问题。
- M1：完成第二轮审查（testing/p2p/world-runtime 优先路径）。
- M2：完成批量优化与回归测试。
- M3：回写文档状态与 devlog 收口。

### Technical Risks
- 审查范围较大，存在漏检风险。
  - 缓解：优先活跃分册与近期变更路径，按“文档 -> 代码 -> 测试”闭环核对。
- 批量优化可能引入回归。
  - 缓解：每项优化配套定向测试与编译校验。

## 第二轮问题清单与优化结果
- R2-01（工程约束，high）
  - design_source：`AGENTS.md`（Rust 单文件不超过 1200 行）。
  - implementation：
    - `crates/oasis7/src/simulator/world_model.rs`（1244）
    - `crates/oasis7/src/simulator/kernel/actions_impl_part1.rs`（1236）
    - `crates/oasis7_consensus/src/quorum.rs`（1233）
  - status：已修复
  - notes：
    - `world_model.rs` 抽离 `PhysicsParameterSpec` 与规格常量到 `world_model/world_model_physics_specs.rs`。
    - `actions_impl_part1.rs` 将 `BuildFactory`/`ScheduleRecipe` 分支下沉为 `part2` 私有方法。
    - `quorum.rs` 测试模块拆分至 `quorum/tests.rs`。
- R2-02（设计一致性，high）
  - design_source：`doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.prd.md`（订阅队列有界）。
  - implementation：`crates/oasis7_consensus/src/network.rs` 在 `publish` 中直接向 `Vec` 追加，存在无界增长风险。
  - status：已修复
  - notes：改为复用 `oasis7_proto::distributed_net::push_bounded_inbox_message`，并新增超限淘汰最旧消息单测。

## 当前状态
- 状态：已完成（2026-02-25）
- 已完成：M0、M1、M2、M3
- 进行中：无
- 阻塞项：无

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
