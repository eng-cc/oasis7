# 文档治理工程化全量优化（2026-02-27）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.design.md`
- 对应需求文档: `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.prd.md`

审计轮次: 4

## 任务拆解
- [x] T0 建档：新增设计文档与项目管理文档
- [x] T1 新增文档治理检查脚本（结构/路径/行数）
- [x] T2 全量修复非 devlog 文档绝对路径为相对路径
- [x] T3 接入 `scripts/ci-tests.sh` required/full 流程
- [x] T4 更新 `testing-manual.md` 文档治理门禁入口
- [x] T5 回归验证、项目文档状态收口、补齐当日 devlog

## 依赖
- `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.design.md`
- `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.prd.md`
- `scripts/ci-tests.sh`
- `testing-manual.md`
- `doc/**/*.project.md`（非 archive）
- `doc/**/*.md`（非 archive / 非 devlog）

## 状态
- 当前阶段：已完成（T0~T5）
- 阻塞项：无
- 最近更新：2026-02-27（完成 T5，项目收口）

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
