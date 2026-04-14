# 仓库文档分工与组织规范（2026-03-09）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/doc-structure-standard.design.md`
- 对应需求文档: `doc/engineering/doc-governance/doc-structure-standard.prd.md`

审计轮次: 4

- 对应规范正文: `doc/engineering/doc-governance/doc-structure-standard.design.md`

## 任务拆解
- [x] T5 (PRD-ENGINEERING-015) [test_tier_required]: 收口 README / legacy redirect 的重复治理话术，明确共享规则回链与最小 redirect 模式。
- [x] T4 (PRD-ENGINEERING-015) [test_tier_required]: 为 ROUND-006 建立“按规范逐文档改造”的执行台账字段与挂靠关系，确保该规范进入实际治理执行面。
- [x] T0 (PRD-ENGINEERING-015) [test_tier_required]: 明确规范目标、范围、非目标与成功标准，完成专题 PRD 建档。
- [x] T1 (PRD-ENGINEERING-015) [test_tier_required]: 输出规范正文，冻结“目录按对象、文件按职责”的组织规则、命名规则（含 `project.md` 作为规范目标命名）、阅读顺序与例外条款。
- [x] T2 (PRD-ENGINEERING-015) [test_tier_required]: 回写 `engineering` 模块入口、索引与主项目文档，保证新规范在文档树中可达。
- [x] T3 (PRD-ENGINEERING-015) [test_tier_required]: 执行文档治理检查并在当日 devlog 留痕。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/README.md`
- `scripts/doc-governance-check.sh`
- `.agents/skills/prd/check.md`

## 状态
- 当前阶段：已完成（T0~T5）
- 阻塞项：无
- 最近更新：2026-04-09
- 说明：本项目文档仅维护该规范专题的执行状态，不处理历史迁移排期。
- ROUND-006 挂靠：`doc/core/reviews/consistency-review-round-006.md` 已将本规范定义为逐文档结构治理轮的唯一裁定依据。
