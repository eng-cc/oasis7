# oasis7: 站点公开发布口径占位（2026-03-11）设计

- 对应需求文档: `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.prd.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.project.md`

审计轮次: 5

## 1. 设计定位
把 `readme` 的 release communication 能力接到站点公开入口，用最小文案占位补齐“说明准备态”。

## 2. 设计结构
- 首页下载区：补“release notes != 正式公告”的说明。
- 文档入口 Hero：补“公开说明仍在准备态”的提示。
- 同步层：CN/EN 首页与 docs hub 四页同构。

## 3. 关键接口 / 入口
- `site/index.html`
- `site/en/index.html`
- `site/doc/cn/index.html`
- `site/doc/en/index.html`
- `doc/readme/governance/readme-release-communication-template-2026-03-11.prd.md`

## 4. 约束与边界
- 不改变“技术预览（尚不可玩）”主结论。
- 不暴露内部评审文档链接。
- 不新增站点脚本，仅做静态文案补位。

## 5. 设计演进计划
- 先补说明准备态占位。
- 再在正式公告发布时替换成正式入口。
- 后续再评估站点自动同步能力。
