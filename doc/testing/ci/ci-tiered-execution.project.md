# oasis7: CI 与提交钩子测试分级（项目管理）

- 对应设计文档: `doc/testing/ci/ci-tiered-execution.design.md`
- 对应需求文档: `doc/testing/ci/ci-tiered-execution.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T1 (PRD-TESTING-CI-TIERED-001/002): 输出设计文档与项目管理文档。
- [x] T2 (PRD-TESTING-CI-TIERED-001/002): 改造 `scripts/ci-tests.sh` 支持 `commit` / `required` / `full`。
- [x] T2.1 (PRD-TESTING-CI-TIERED-001): 调整 `scripts/pre-commit.sh` 默认跑 `commit` baseline。
- [x] T3 (PRD-TESTING-CI-TIERED-002/003): 调整 `.github/workflows/rust.yml`，push/PR 跑 `required`，每日定时跑 `full`。
- [x] T4 (PRD-TESTING-CI-TIERED-002/003): 文档回写、任务日志更新、验证并提交。
- [x] T5 (PRD-TESTING-004): 专题文档人工迁移到 strict schema，并切换命名为 `.prd.md/.project.md`。

## 依赖
- doc/testing/ci/ci-tiered-execution.prd.md
- 统一测试入口：`scripts/ci-tests.sh`
- 本地提交入口：`scripts/pre-commit.sh`
- GitHub Actions workflow：`.github/workflows/rust.yml`
- 模块主追踪文档：
  - `doc/testing/prd.md`
  - `doc/testing/project.md`

## 状态
- 更新日期：2026-04-10
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（当前专题已收口）
- 审计备注（2026-03-05 ROUND-002）：本文件仅保留执行任务记录；规则定义以 `ci-tiered-execution.prd.md` 为准。
- 补充备注（2026-04-10）：默认本地提交路径已从 `required` 收紧为 `commit` baseline；`required` 继续保留为显式本地重门禁与 PR/CI required gate。
