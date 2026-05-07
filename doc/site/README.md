# site 文档索引

审计轮次: 10

## 从这里开始
- 想先看公开 docs hub 入口与对外阅读路径：`site/doc/cn/index.html` / `site/doc/en/index.html`
- 想直接拿公开 `oasis7` skill 链接：`site/skills/oasis7.md`
- 想先理解站点模块边界、同步原则与验收口径：`doc/site/prd.md`
- 想先看当前站点任务、同步状态与最近完成项：`doc/site/project.md`
- 想直接按文件名定位某个 github-pages / manual 专题：`doc/site/prd.index.md`
- 想先确认静态手册镜像如何挂到仓库权威文档：`doc/site/manual/site-manual-static-docs.prd.md` 与 `doc/world-simulator/viewer/viewer-manual.manual.md`
- 想先确认下载链路、公开公告占位与真实状态口径：`doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md` 与 `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.prd.md`

## 入口
- PRD: `doc/site/prd.md`
- 设计总览: `doc/site/design.md`
- 标准执行入口: `doc/site/project.md`
- 文件级索引: `doc/site/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：帮助读者先决定去模块 PRD、项目台账、文件级索引，还是少量仍承担当前公开口径判断职责的高频专题。
- `site/doc/{cn,en}/index.html` 是对外 docs hub，承担公开阅读入口；仓库内 `doc/site/**` 负责治理规则、同步策略与追溯。
- `site/skills/oasis7.md` 是公开 `oasis7` skill 的可直接抓取 Markdown 镜像；中英 docs hub 只负责给它提供入口。
- `doc/site/manual/` 负责静态手册镜像策略与 canonical/manual 映射，不替代公开页面本身。
- `doc/world-simulator/viewer/viewer-manual.manual.md` 仍是仓库内 canonical Viewer 手册，`site/doc/{cn,en}/viewer-manual.html` 只是公开只读镜像。
- `doc/site/prd.index.md` 是精确检索索引，适合已经知道专题名或需要完整文件清单时使用，不适合作为第一次进入模块时的首读入口。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再把 `github-pages/` 与 `manual/` 的专题长名单直接平铺在首屏。
- 默认活跃入口保留在 `doc/site/prd.md`、`doc/site/project.md`、`doc/site/prd.index.md` 与少量仍承担当前公开状态判断职责的正式专题。
- 公开 HTML 镜像、补充手册页面和历史专题继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径按需进入。

## 模块职责
- 维护公开首页、docs hub、下载入口与公开叙事边界。
- 维护可直接抓取的 raw Markdown skill 分发入口。
- 维护 github-pages 子域下的站点结构、内容同步、CTA、发布流水线与质量门禁专题。
- 维护 manual 子域下的静态文档站与 Viewer 手册镜像策略。
- 承接公开“技术预览 / not playable yet / diagnostics only”口径与仓库 canonical 文档之间的一致性。

## 热点子域导航（2026-04-11 快照）
- `github-pages/` 正式专题三件套（54）：公开首页、下载链路、公告占位、质量门禁、内容同步与公开叙事边界。
- `manual/` 正式专题三件套（6）：静态文档站与 Viewer 手册镜像策略。
- 模块根入口（5）：`README.md`、`prd.md`、`project.md`、`design.md`、`prd.index.md`。
- 公开 HTML 入口（仓库外显层）：`site/index.html`、`site/en/index.html`、`site/doc/{cn,en}/index.html`、`site/doc/{cn,en}/viewer-manual.html`。
- 公开 raw skill 入口（可直接抓取）：`site/skills/oasis7.md`。

## 高密度提示
- `doc/site/` 当前共有 67 份文件，其中 `doc/site/github-pages/` 占 56 份；默认入口不再尝试把 github-pages 长表直接摊平到模块首页。
- 需要完整活跃专题清单时，进入 `doc/site/prd.index.md`；需要公开 docs hub、下载页或手册镜像时，再按 `site/**` 的公开页面定向进入。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 站点信息架构、公开状态口径或镜像入口变化时，优先更新 `doc/site/prd.md` / `doc/site/project.md`；新增默认首读入口或专题后，再同步回写 `doc/site/prd.index.md` 与本页“从这里开始”。
