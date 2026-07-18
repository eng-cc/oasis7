# 玩家普通用户发行体验产品契约（2026-04-14）

- 父模块 PRD: [`玩家入口与发行 PRD`](prd.md)
- 产品模块总入口: [`doc/product/README.md`](../README.md)
- 文档类型：活跃产品专题 PRD（不分配并列 Product PRD-ID）
- 产品追踪边界: [`玩家普通用户发行体验产品契约追踪`](player-release-distribution-contract-2026-04-14.project.md)
- 对应项目管理文档: `doc/product/player-entry-distribution/player-release-distribution-contract-2026-04-14.project.md`
- 下层专业域：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`发布入口 + Release 安装包流水线`](../../site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md)

## 目标

玩家应能按自己的平台看到一个推荐下载包和主 CTA，在下载前看见支持边界、版本与 checksum，并能直接安装或启动，而不必研究 shell、bundle 内容或多个技术资产的差异。

## 范围

当前升级只支持重新下载最新主包并手动覆盖安装/替换；用户应先备份 `config.toml`、`.oasis7_launcher_ux_state.json` 与 `output/chain-runtime/<node_id>/reward-runtime-execution-world/`。这不是应用内更新、自动迁移或跨目录状态保留承诺。

## 接口 / 数据

玩家入口、资产和发布证据只通过下层专业域权威下钻；本产品专题不定义资产字段、工作流输入或安装器接口。

## 里程碑

本专题完成将普通用户发行的产品体验边界归入 `player-entry-distribution`；后续签名、notarization 和发行实现以专业域与 GitHub task truth 的独立证据为准。

## 风险

普通用户广泛发行仍被阻断：Windows 需要完成 codesigning trust chain，macOS 需要完成签名和 notarization。产品层不得把现有技术预览资产、单主 CTA 或手动安装路径表述为已完成的无安全提示普通用户发行。

## 权威与验收边界

本专题只拥有玩家下载、安装、升级边界与可理解性的产品结果。`world-simulator` 拥有 Launcher、资产入口、安装器验证、签名/notarization 失败语义与技术门禁；站点 release-pipeline 文档拥有资产名、工作流、校验、状态路径和执行记录。根 `README.md` 仍是公开状态与 claim envelope 的唯一权威。

产品结果成立时，玩家能够：

1. 按平台选择一个推荐入口，并在下载前确认支持边界、版本和 checksum；
2. 无需 shell 或 bundle 考古即可完成受支持的安装/启动；
3. 明确理解当前手动替换与备份边界；
4. 不把未完成 Windows/macOS 信任链误读为普通用户广泛发行已就绪。
