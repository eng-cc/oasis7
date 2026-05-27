# 文档迁移并行协作方案（2026-03-03）项目管理文档

- 对应设计文档: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.design.md`
- 对应需求文档: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-010 (PRD-ENGINEERING-005): 冻结四人并行迁移原则、目录边界与执行流程，并产出协作主文档。
- [x] TASK-ENGINEERING-011 (PRD-ENGINEERING-006): Owner-A 迁移 `doc/world-simulator/**` 待迁移文档（146 篇）。
- [x] TASK-ENGINEERING-012 (PRD-ENGINEERING-006): Owner-B 迁移 `doc/p2p/**` 待迁移文档（70 篇）。
- [x] TASK-ENGINEERING-013 (PRD-ENGINEERING-006): Owner-C 迁移 `doc/world-runtime/**`、`doc/headless-runtime/**` 待迁移文档（30 篇）。
- [x] TASK-ENGINEERING-013B (PRD-ENGINEERING-006): Owner-C Batch-C2 迁移 `doc/headless-runtime/**` 待迁移文档（4 篇）。
- [x] TASK-ENGINEERING-013C (PRD-ENGINEERING-006): Owner-C Batch-C3 迁移 `doc/world-runtime/governance/**`、`doc/world-runtime/module/**`、`doc/world-runtime/wasm/**` 待迁移文档（9 篇）。
- [x] TASK-ENGINEERING-013D (PRD-ENGINEERING-006): Owner-C Batch-C4 迁移 `doc/world-runtime/runtime/**` 待迁移文档（17 篇）。
- [x] TASK-ENGINEERING-014 (PRD-ENGINEERING-006): Owner-D 迁移 `doc/site/**`、`doc/readme/**`、`doc/scripts/**`、`doc/game/**`、`doc/engineering/**` 及根入口遗留文档（57 篇；D1/D2 已完成）。
- [x] TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006): Owner-D 已完成非根入口 54 篇迁移（不含 3 份根入口 redirect project 文档）。
- [x] TASK-ENGINEERING-014-D2 (PRD-ENGINEERING-006): Owner-D 完成 3 份根入口 redirect project 文档收口（`doc/game-test.project.md`、`doc/world-runtime.project.md`、`doc/world-simulator.project.md`）。
- [x] TASK-ENGINEERING-015 (PRD-ENGINEERING-007): 执行全量收口复核（命名一致性、引用可达、模块追踪同步、燃尽归零）。

## 依赖
- `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.design.md`
- `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`
- `doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md`
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/devlog/README.md`
- `./scripts/doc-governance-check.sh`

## 状态
- 更新日期: 2026-03-11
- 当前状态: completed
- 当前完成: 11 / 11（完成协作入口冻结 + Owner-A/B/C/D 全部迁移批次 + 根入口 redirect 收口 + 全量收口复核）
- 下一任务: 无（当前迁移协作子项目已完成）
- 风险备注: 迁移专项已收口；后续若新增 legacy 迁移需求，应新开专题而非回滚本子项目完成态。
