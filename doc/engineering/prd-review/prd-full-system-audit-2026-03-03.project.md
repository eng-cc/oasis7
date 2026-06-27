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

## 已读清单（逐篇）
- 模块审读清单（文件名沿用 `active-*` 以兼容 2026-03 审计与 review 引用；清单不是当前模块活跃入口或最新专题索引）：
  - `doc/engineering/prd-review/checklists/active-core.md`
  - `doc/engineering/prd-review/checklists/active-engineering.md`
  - `doc/engineering/prd-review/checklists/active-game.md`
  - `doc/engineering/prd-review/checklists/active-headless-runtime.md`
  - `doc/engineering/prd-review/checklists/active-p2p.md`
  - `doc/engineering/prd-review/checklists/active-playability_test_result.md`
  - `doc/engineering/prd-review/checklists/active-readme.md`
  - `doc/engineering/prd-review/checklists/active-scripts.md`
  - `doc/engineering/prd-review/checklists/active-site.md`
  - `doc/engineering/prd-review/checklists/active-testing.md`
  - `doc/engineering/prd-review/checklists/active-world-runtime.md`
  - `doc/engineering/prd-review/checklists/historical-world-simulator-2026-03-05.md`（historical snapshot; current world-simulator truth is routed through `doc/world-simulator/README.md` and `doc/world-simulator/prd.index.md`）
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
- 当前状态: active
- 当前完成: 5 / 5（全量 PRD 708 篇已完成逐篇审读与清单回填）
- 下一任务: 周度增量巡检（沿用 TASK-ENGINEERING-024 机制）
- 本轮发现与修复:
  - 修复 `doc/core/prd.md` 中 `game-test` 路径为 `.prd` 命名入口。
  - 修复 79 个唯一旧路径引用（共 179 处回写），覆盖历史链路与活跃链路。
  - 补齐 `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md` 与 `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md` 到已读清单，清单覆盖率提升到 708/708。
