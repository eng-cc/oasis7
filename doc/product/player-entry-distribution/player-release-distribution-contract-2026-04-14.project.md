# 玩家普通用户发行体验产品契约追踪（2026-04-14）

- 父模块 PRD: [`玩家入口与发行 PRD`](prd.md)
- 产品模块总入口: [`doc/product/README.md`](../README.md)
- 对应产品专题 PRD: [`玩家普通用户发行体验产品契约`](player-release-distribution-contract-2026-04-14.prd.md)
- 对应需求文档: `doc/product/player-entry-distribution/player-release-distribution-contract-2026-04-14.prd.md`
- 专业域权威: [`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`发布入口 + Release 安装包流水线`](../../site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md)

## 产品专题追踪边界

本文件只记录产品专题的迁移状态、跨域 authority 和阻断摘要；不维护发行实现任务、安装器检查表、测试命令、发布处置或任务状态。那些真值继续由上述专业域文档及 GitHub task issue evidence comments 承载。

## 任务拆解

产品层不维护本专题的本地任务清单。发行实现、验证和发布处置由绑定 GitHub task issue evidence comments 与专业模块 `project.md` 拆解；本专题只在产品承诺、跨域验收或 blocker 语义变化时更新。

## 依赖

- Launcher、资产入口、安装器验证和技术门禁：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)
- 资产名、发布工作流、校验、状态路径和执行记录：[`发布入口 + Release 安装包流水线`](../../site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md) 及其 [project](../../site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md)

## 状态

本专题已承接原 launcher 普通用户发行主题的玩家体验边界：每个平台一个推荐入口、下载前支持信息、手动替换与备份边界。普通用户广泛发行仍受 Windows codesigning trust chain 与 macOS 签名/notarization 阻断；它们是专业实现与验证事项，不由本产品专题裁决。

## 权威与任务证据

- 本次文档迁移证据：GitHub task [`#2433`](https://github.com/eng-cc/oasis7/issues/2433)

产品承诺、跨域验收或 blocker 语义变化时更新本专题 PRD；实现、验证或发布任务变化时仅更新对应专业域与 GitHub-backed task truth。
