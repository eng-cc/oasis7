# oasis7: 公告 / Changelog 模板（2026-03-11）设计

- 对应需求文档: `doc/readme/governance/readme-release-announcement-template-2026-03-11.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-announcement-template-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
把首份 announcement / changelog 底稿抽象成固定模板，供后续候选直接实例化。

## 2. 设计结构
- Header：标题、候选 ID、状态、来源。
- Narrative：Summary / What This Means / Highlights / Limitations。
- FAQ / Next Steps：答疑与动作。
- Approval：source links、owner、review status。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-release-announcement-template-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-announcement-template-2026-03-11.project.md`
- `doc/readme/governance/readme-release-announcement-template-2026-03-11.md`
- `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md`

## 4. 约束与边界
- 模板不包含具体候选内容。
- 模板必须保留 source-link 与 review status。
- 模板服务 draft 底稿，不替代最终发布文案。

## 5. 设计演进计划
- 先冻结模板结构。
- 再在新候选中复用验证。
- 后续视需要接入更正式的公告体系。
