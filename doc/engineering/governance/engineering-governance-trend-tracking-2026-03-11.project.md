# oasis7: 工程门禁趋势跟踪（2026-03-11）（项目管理）

- 对应设计文档: `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.design.md`
- 对应需求文档: `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] EGT-1 (PRD-ENGINEERING-TREND-001/002) [test_tier_required]: 建立专题 PRD / Design / Project，冻结样本字段、指标公式与红黄绿阈值。
- [x] EGT-2 (PRD-ENGINEERING-TREND-002/003) [test_tier_required]: 生成首份 baseline，纳入至少 3 个近期 engineering 门禁样本并回写结论。
- [x] EGT-3 (PRD-ENGINEERING-TREND-001/003) [test_tier_required]: 完成 `producer_system_designer -> qa_engineer` handoff，并回写模块主项目与 devlog。

## 依赖
- `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md`
- `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.design.md`
- `doc/engineering/evidence/engineering-governance-trend-baseline-2026-03-11.md`
- `doc/engineering/governance/producer-to-qa-task-engineering-003-governance-trend-2026-03-11.md`
- `doc/engineering/project.md`
- `doc/devlog/README.md`
- `doc/devlog/README.md`
- `scripts/doc-governance-check.sh`

## 状态
- 更新日期：2026-03-11
- 当前阶段：已完成
- 阻塞项：无
- 下一步：在 `TASK-ENGINEERING-004` 中复用本 baseline，建立季度审查模板与续写节奏。
