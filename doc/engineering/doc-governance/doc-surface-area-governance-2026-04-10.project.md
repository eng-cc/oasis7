# 文档体量治理与活跃阅读面收敛（2026-04-10）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.design.md`
- 对应需求文档: `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.prd.md`

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
- `doc/engineering/doc-governance/doc-structure-standard.prd.md`
- `doc/engineering/doc-governance/doc-structure-standard.design.md`
- `doc/.governance/module-root-md-allowlist.txt`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: 已完成（T0~T4）
- 阻塞项: 无
- 最近更新: 2026-04-14
- 后续动作: `world-simulator / p2p / testing / readme / core / world-runtime / game / site` 的高密度模块入口减重，以及 `engineering / scripts / playability_test_result / headless-runtime` 的低密度模块复核均已回到 engineering 主项目收口。当前专题不再继续堆叠模块级任务；后续仅在入口再次失去 `what / where / next / risk` 首读分流能力时，才重开新一轮减重，否则转入路径级治理、round 追加或脚本门禁扩展评估。
