# oasis7: 版本候选公告 / Changelog 底稿（2026-03-11）设计

- 对应需求文档: `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-announcement-draft-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
将 communication brief 翻译成更接近外部公告 / changelog 的底稿层，同时保留 draft 状态与审核边界。

## 2. 设计结构
- Header：标题、候选 ID、状态、来源。
- Summary：本轮亮点与当前结论。
- Non-Promise / Risk：未承诺项与限制。
- FAQ / Next Steps：运营答疑与下一步。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-announcement-draft-2026-03-11.project.md`
- `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md`
- `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`

## 4. 约束与边界
- 必须显式标注 `draft`。
- 不直接替代正式发布公告。
- 不写超出 brief / go-no-go 的承诺。

## 5. 设计演进计划
- 先形成首份底稿。
- 再视需要抽象公告专用模板。
- 最后评估是否联动站点或 README 公开面。
