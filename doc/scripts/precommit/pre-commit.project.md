# Pre-commit Checks（本地提交前测试脚本）（项目管理文档）

- 对应需求文档: `doc/scripts/precommit/pre-commit.prd.md`

审计轮次: 4

## 任务拆解
- [x] 输出设计文档（`doc/scripts/precommit/pre-commit.prd.md`）
- [x] 输出项目管理文档（本文件）
- [x] 新增本地提交前联测脚本（`scripts/pre-commit.sh`）
- [x] 安装 git pre-commit hook（调用 `scripts/pre-commit.sh`）
- [x] 更新任务日志
- [x] 运行测试 `./scripts/pre-commit.sh`
- [x] 提交到 git
- [x] 对齐 CI 测试清单（改为调用 `scripts/ci-tests.sh`）
- [x] 提交前新增代码格式化时机（`cargo fmt --all`）
- [x] CI 增加格式化检查（`cargo fmt --all -- --check`）
- [x] 文档补充：新仓库需重新注册 pre-commit hook（2026-02-07）
- [x] 移除默认 pre-commit/CI 的 builtin wasm hash 校验（改为手动按需执行，2026-02-14）
- [x] 历史方案变更：不再将 builtin wasm hash 校验放在 required 基础门禁（以 `scripts/ci-tests.sh` 当前行为为准）
- [x] pre-commit 增加 viewer wasm32 编译检查（`cargo check -p oasis7_viewer --target wasm32-unknown-unknown`，2026-02-15）
- [x] 修复提交钩子 `fmt` 失败并恢复全链路通过（`cargo fmt --all` + 补齐 `selection_linking` 新事件分支，2026-02-16）
- [x] 修复提交钩子 `fmt` 失败并恢复全链路通过（`cargo fmt --all` 修复 `node_points` / `node_points_runtime` 格式漂移，2026-02-16）
- [x] 修复 CI `cargo fmt --check` 漂移并恢复门禁通过（`oasis7_viewer_live*` / `oasis7_node*`，2026-02-17）
- [x] 清理 required 门禁 warning（`oasis7_viewer_live*` DistFS/Node 导入与测试辅助函数，2026-02-17）
- [x] required 门禁口径同步到当前实现：`doc-governance + fmt + required tier + consensus/distfs/viewer + viewer wasm check`（2026-03-04）
- [x] builtin wasm-heavy runtime 用例从 required 移到 `test_tier_full`，恢复 pre-commit 轻量 required 口径（2026-04-10）
- [x] 将 `cargo test -p oasis7 --tests --features test_tier_required` 从默认 `pre-commit` 路径拆到显式 `./scripts/ci-tests.sh required`，新增 `commit` tier 作为本地提交基线（2026-04-10）
- [x] 将 `cargo test -p oasis7_viewer` 与 `cargo check -p oasis7_viewer --target wasm32-unknown-unknown` 从默认 `commit` tier 移出，仅保留在显式 `required` / CI required gate（2026-04-10）

## 依赖
- `rustfmt`（staged `.rs`）/ `cargo fmt -- --check`
- `cargo test`（oasis7 viewer 联测）
- `wasm32-unknown-unknown` Rust target（viewer wasm 编译检查）

## 状态
- 当前阶段：已提交
- 最近更新：`scripts/pre-commit.sh` 已改为静默成功 no-op，普通 commit 不再运行格式化或验证；frozen-head Pre-PR Ready 与 CI required gate 保持 authoritative（2026-07-14）。更早 commit-tier 演进记录保留在上方历史完成项。
- 审计备注（2026-03-05 ROUND-002）：本文件仅保留执行记录；required/full 规则定义与命令矩阵以 `ci-tiered-execution`、`ci-testcase-tiering`、`ci-test-coverage` 及 `scripts/ci-tests.sh` 为准。
- 合并备注（2026-07-16）：原 `precommit-remediation-playbook.project.md` 的完成事项已并入上方任务记录；当前修复流程以 `pre-commit.prd.md` 为准。更早路径只在历史审计记录中作 provenance 保留。
- 实现记录：`scripts/fix-precommit.sh` 于 2026-02-06 完成；2026-07-16 将其当前操作契约从 remediation 叶子收口到 `pre-commit.prd.md`。

### Remediation 完成记录（历史）

- 完成事项：输出 remediation 需求与设计文档。
- 完成事项：新增并验证 `scripts/fix-precommit.sh`。
- 完成事项：将完成结果回写 `doc/devlog/README.md`。
- 历史实现依赖：`scripts/pre-commit.sh`、`scripts/ci-tests.sh`、`rustfmt` / `cargo fmt`。
- 完成状态：M3，无后续阶段；`fix-precommit` 全链路校验于 2026-02-06 通过。
- 审计备注（2026-03-05 ROUND-002）：当时的执行记录保留在 project 文档；2026-07-16 起，当前失败修复流程改由 `pre-commit.prd.md` 统一维护。
- 迁移记录（2026-03-03）：曾按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`，任务拆解、依赖与完成结论随本次合并继续保留。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
