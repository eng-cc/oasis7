# 文档体量治理与活跃阅读面收敛（2026-04-10）项目管理文档

- 对应设计文档: `doc/engineering/doc-surface-area-governance-2026-04-10.design.md`
- 对应需求文档: `doc/engineering/doc-surface-area-governance-2026-04-10.prd.md`

审计轮次: 1

## 任务拆解
- [x] T0 (PRD-ENGINEERING-024) [test_tier_required]: 形成专题 PRD，冻结问题定义、四层消费模型、成功标准与非目标。
- [x] T1 (PRD-ENGINEERING-024) [test_tier_required]: 输出专题设计文档，定义默认阅读面、密度触发器、优先级与执行顺序。
- [x] T2 (PRD-ENGINEERING-024) [test_tier_required]: 回写 engineering 主 PRD、主项目、README 与 `prd.index.md`，让专题在模块入口可达。
- [x] T3 (PRD-ENGINEERING-024) [test_tier_required]: 回写 `.pm` task execution log 与 task 元数据，确保专题与执行证据互链。
- [x] T4 (PRD-ENGINEERING-024) [test_tier_required]: 更新 `module-root-md-allowlist` 并执行 `scripts/doc-governance-check.sh`，验证本批新增根级专题文件通过门禁。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/doc-structure-standard.prd.md`
- `doc/engineering/doc-structure-standard.design.md`
- `doc/.governance/module-root-md-allowlist.txt`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: 已完成（T0~T4）
- 阻塞项: 无
- 最近更新: 2026-04-10
- 后续动作: 按本专题冻结的优先级，为 `world-simulator / p2p / testing` 拆首批活跃阅读面收敛任务；这些任务回到 engineering 主项目继续追踪，不在本专题 project 内继续堆叠。
