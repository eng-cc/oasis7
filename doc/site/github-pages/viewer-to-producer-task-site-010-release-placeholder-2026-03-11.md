# Role Handoff Brief

审计轮次: 5

## Meta
- Handoff ID: `HANDOFF-SITE-010-2026-03-11-RELEASE-PLACEHOLDER`
- Date: `2026-03-11`
- From Role: `viewer_engineer`
- To Role: `producer_system_designer`
- Related PRD-ID: `PRD-SITE-005/006/007`
- Related Task ID: `TASK-SITE-010`
- Priority: `P1`

## Goal
- 交付站点公开入口的“说明准备态”占位文案，使公开站点开始承接 release communication 链，但不越过技术预览与正式发布边界。

## Why Now
- `readme` 已完成简报、底稿与模板；若站点仍没有对应占位，公开入口会继续只展示下载/技术预览层，而缺失发布沟通状态。

## Inputs
- 代码 / 文档入口：`site/index.html`、`site/en/index.html`、`site/doc/cn/index.html`、`site/doc/en/index.html`
- 已完成内容：站点已统一“技术预览（尚不可玩）”与 CTA 优先级
- 已知约束：不新增脚本、不暴露内部评审文档链接
- 依赖前置项：`TASK-SITE-008/009` 与 `TASK-README-006/007/008/009`

## Expected Output
- 接收方交付物 1：确认公开入口已补“正式公告仍在准备态”的安全说明
- 接收方交付物 2：后续若正式公告上线，可沿用当前占位位置直接替换
- 需要回写的文档 / 日志：`doc/site/project.md`、`doc/devlog/README.md`

## Done Definition
- [x] 四个公开入口页已同构补位
- [x] 未削弱“技术预览（尚不可玩）”主口径
- [x] 补齐测试 / 验证证据

## Risks / Blockers
- 风险：若未来正式公告未替换该占位，站点会长期停留在“准备态”描述
- 阻断项：无
- 需要升级给谁：`liveops_community`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n '公开说明：正式公告仍在准备中|Public note: formal announcement is still being prepared|当前下载页的“发布说明”用于构建说明|Current release-notes links describe preview build artifacts' site/index.html site/en/index.html site/doc/cn/index.html site/doc/en/index.html`

## Notes
- 接收方确认范围：`已接收站点公开口径占位结果；后续正式发布时可直接替换该位置`
- 接收方确认 ETA：`same-day`
- 接收方新增风险：`无`
