# site 文档索引

审计轮次: 9

## 从这里开始
- 想看公开 docs hub 入口与对外阅读路径：`site/doc/cn/index.html` / `site/doc/en/index.html`
- 想理解站点模块的边界、同步原则与验收口径：`doc/site/prd.md`
- 想看当前站点任务、同步状态与最近完成项：`doc/site/project.md`
- 想按专题查具体站点设计文档：`doc/site/prd.index.md`
- 想确认静态手册镜像如何挂到仓库权威文档：`doc/site/manual/` + `doc/world-simulator/viewer/viewer-manual.manual.md`

## 入口
- PRD: `doc/site/prd.md`
- 设计总览: `doc/site/design.md`
- 标准执行入口: `doc/site/project.md`
- 文件级索引: `doc/site/prd.index.md`

## 入口分工
- `README.md` 只负责说明 site 模块与公开站点的映射关系，不替代公开页面本身。
- `site/doc/{cn,en}/index.html` 是对外 docs hub，承担公开阅读入口；仓库内 `doc/site/**` 负责治理规则、同步策略与追溯。
- `doc/site/manual/` 维护静态站/手册镜像策略；具体公开镜像页面仍落在 `site/doc/**`。
- `doc/world-simulator/viewer/viewer-manual.manual.md` 仍是仓库内 canonical Viewer 手册，`site/doc/{cn,en}/viewer-manual.html` 只是公开只读镜像。

## 主题文档
- `github-pages/`：站点结构、内容同步、CTA、发布流水线与公开发布口径占位。
- `manual/`：静态文档站与手册迁移维护策略。

## 近期专题
- `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`
- `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.prd.md`
- `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`
- `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md`

## 根目录收口
- 模块根目录主入口保留：`README.md`、`prd.md`、`design.md`、`project.md`、`prd.index.md`。
- 其余专题文档按主题下沉到 `github-pages/`、`manual/`。

## 维护约定
- 站点信息架构、公开状态口径与发布说明变更需同步更新 PRD、项目状态与公开页面。
- 新增 `github-pages` / `manual` 专题后，需同步回写 `doc/site/prd.index.md` 与本目录索引。
- 若公开 docs hub 的入口分流或 Viewer 镜像链接变化，需同时核对 `site/doc/{cn,en}/index.html`、`doc/site/README.md` 与仓库内 canonical 手册入口是否仍一致。
