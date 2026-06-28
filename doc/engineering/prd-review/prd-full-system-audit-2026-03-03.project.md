# 全量 PRD 体系审读与对齐（2026-03-03）项目管理文档

- 对应设计文档: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.design.md`
- 对应需求文档: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-020 (PRD-ENGINEERING-012) [test_tier_required]: 建立全量 PRD 逐篇审读机制，生成已读清单并完成模块入口三件套首批审读。
- [x] TASK-ENGINEERING-021 (PRD-ENGINEERING-013) [test_tier_required]: 逐模块核对专题文档与代码一致性；发现偏差按代码回写并补充处理动作。
- [x] TASK-ENGINEERING-022 (PRD-ENGINEERING-013/014) [test_tier_required]: 审查跨文档重复与上下游口径漂移，执行合并/重定向/引用修复。
- [x] TASK-ENGINEERING-023 (PRD-ENGINEERING-014) [test_tier_required]: 清理 archive 目录与历史引用收口，补齐替代链对齐。
- [x] TASK-ENGINEERING-024 (PRD-ENGINEERING-012/013/014) [test_tier_required]: 建立周度增量审读节奏（新增/变更 PRD 自动入清单）。

> Historical note: 上述 `TASK-ENGINEERING-024` 描述的是 2026-03 当期审读机制；当前不再生成旧清单文件，新增/变更文档通过模块 `README.md` / `prd.index.md` / `project.md` / `prd.md` 与 round review logs 追踪。

## 历史已读留痕（逐篇）
- 模块审读 snapshot（历史条目沿用当期模块名以兼容 2026-03 审计与 review 引用；snapshot 不是当前模块活跃入口或最新专题索引）：
  - historical core PRD review checklist snapshot（后续已删除；当前 core truth 见 `doc/core/README.md`、`doc/core/prd.index.md` 与 `doc/core/project.md`）
  - historical engineering PRD review checklist snapshot（后续已删除；当前 engineering truth 见 `doc/engineering/README.md`、`doc/engineering/prd.index.md` 与 `doc/engineering/project.md`）
  - historical game PRD review checklist snapshot（后续已删除；当前 game truth 见 `doc/game/README.md`、`doc/game/prd.index.md` 与 `doc/game/project.md`）
  - historical headless-runtime PRD review checklist snapshot（后续已删除；当前 headless-runtime truth 见 `doc/headless-runtime/README.md`、`doc/headless-runtime/prd.index.md` 与 `doc/headless-runtime/project.md`）
  - historical p2p PRD review checklist snapshot（后续已删除；当前 p2p truth 见 `doc/p2p/README.md`、`doc/p2p/prd.index.md` 与 `doc/p2p/project.md`）
  - historical playability_test_result PRD review checklist snapshot（后续已删除；当前 playability_test_result truth 见 `doc/playability_test_result/README.md`、`doc/playability_test_result/prd.index.md` 与 `doc/playability_test_result/project.md`）
  - historical readme PRD review checklist snapshot（后续已删除；当前 readme truth 见 `doc/readme/README.md`、`doc/readme/prd.index.md`、`doc/readme/project.md` 与 `doc/readme/prd.md`）
  - historical scripts PRD review checklist snapshot（后续已删除；当前 scripts truth 见 `doc/scripts/README.md`、`doc/scripts/prd.index.md`、`doc/scripts/project.md` 与 `doc/scripts/prd.md`）
  - historical site PRD review checklist snapshot（后续已删除；当前 site truth 见 `doc/site/README.md`、`doc/site/prd.index.md`、`doc/site/project.md` 与 `doc/site/prd.md`）
  - historical testing PRD review checklist snapshot（后续已删除；当前 testing truth 见 `doc/testing/README.md`、`doc/testing/prd.index.md`、`doc/testing/project.md` 与 `doc/testing/prd.md`）
  - historical world-runtime PRD review checklist snapshot（后续已删除；当前 world-runtime truth 见 `doc/world-runtime/README.md`、`doc/world-runtime/prd.index.md` 与 `doc/world-runtime/project.md`）
  - historical world-simulator PRD review checklist snapshot（后续已删除；当前 world-simulator truth 见 `doc/world-simulator/README.md`、`doc/world-simulator/prd.index.md`、`doc/world-simulator/project.md` 与 `doc/world-simulator/prd.md`）
  - root legacy redirect checklist（后续已删除；root PRD/project shells no longer have active review targets）
## 依赖
- `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.design.md`
- `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md`
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/*/prd.md`
- `doc/*/project.md`
- `doc/*/prd.index.md`
- `scripts/doc-governance-check.sh`
- `scripts/site-manual-sync-check.sh`

## 状态
- 更新日期: 2026-03-05
- 当前状态: done; historical audit evidence only
- 当前完成: 5 / 5（全量 PRD 708 篇已完成逐篇审读与清单回填）
- 下一任务: 无当前 checklist 机制后续；新增/变更文档按模块 `README.md` / `prd.index.md` / `project.md` / `prd.md` 与 round review logs 追踪。
- 本轮发现与修复:
  - 修复 `doc/core/prd.md` 中 `game-test` 路径为 `.prd` 命名入口。
  - 修复 79 个唯一旧路径引用（共 179 处回写），覆盖历史链路与活跃链路。
  - 补齐 `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md` 与 `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md` 到当期已读留痕，历史覆盖率提升到 708/708。
