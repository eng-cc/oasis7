# 系统性应用测试手册工程化收口（项目管理文档）

- 对应设计文档: `doc/testing/manual/systematic-application-testing-manual.design.md`
- 对应需求文档: `doc/testing/manual/systematic-application-testing-manual.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] TMAN-1 (PRD-TESTING-MANUAL-001): 完成手册迁移与主入口命名统一（`testing-manual.md`）。
- [x] TMAN-2 (PRD-TESTING-MANUAL-001/002): 收口分层模型（L0~L5）与套件矩阵（S0~S10）。
- [x] TMAN-3 (PRD-TESTING-MANUAL-002/003): 完成 Web 闭环分册拆分并建立主手册引用入口。
- [x] TMAN-4 (PRD-TESTING-MANUAL-002/003): 按 CI 与执行经验持续补齐 fail-fast、GPU/headed 门禁与运行约束。
- [x] TMAN-5 (PRD-TESTING-004): 专题文档人工迁移到 strict schema 并统一 `.prd.md/.project.md` 命名。

## 依赖
- doc/testing/manual/systematic-application-testing-manual.prd.md
- `testing-manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
- `scripts/ci-tests.sh`
- `scripts/viewer-primary-web-entry-regression.sh`
- `scripts/viewer-software-safe-step-regression.sh`
- `scripts/viewer-software-safe-chat-regression.sh`
- `doc/testing/prd.md`
- `doc/testing/project.md`

## 状态
- 更新日期：2026-03-03
- 当前阶段：已完成
- 阻塞项：无
- 下一步：无（manual 批次迁移已收口）
