# site PRD 文件级索引

审计轮次: 7

更新时间：2026-04-11

## 入口
- 模块 PRD：`doc/site/prd.md`
- 模块设计总览：`doc/site/design.md`
- 模块标准执行入口：`doc/site/project.md`
- 当前高频 site 入口：`doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`

## 首读分流
- 想先回答 site 模块在管什么、哪些公开边界是当前真值：先读 `doc/site/prd.md`
- 想先回答当前站点同步状态、最近完成项和是否还有未收口任务：先读 `doc/site/project.md`
- 想先看公开 docs hub 与手册镜像的 canonical 策略：先读 `doc/site/manual/site-manual-static-docs.prd.md`
- 想先看 GitHub Pages 下载链路与公开版本说明边界：先读 `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`
- 想先看“正式公告仍在准备中”的公开口径占位：先读 `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.prd.md`
- 想继续按子域或文件名下钻：使用下方密度快照、热点子域导航与补充入口

## 密度快照（2026-04-11）
- `doc/site/`：67 份文件
- `doc/site/github-pages/`：56 份文件
- `doc/site/manual/`：6 份文件
- 模块根入口：5 份文件
- `doc/site/` 正式专题三件套：60 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `github-pages/` 正式专题三件套 | 54 | 公开首页、下载链路、公告占位、CTA、内容同步、SEO 与质量门禁 |
| `manual/` 正式专题三件套 | 6 | 静态 docs hub、Viewer 手册镜像与 canonical/manual 映射 |
| 模块根入口 | 5 | 模块目标态、执行台账、设计总览与文件级精确检索 |

## 活跃补充文档
- `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`：GitHub Pages 下载链路、发布资产和公开版本说明主入口。
- `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.prd.md`：公开公告占位与 technical preview 边界主入口。
- `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`：站点主叙事、docs hub 与 game-first 入口重定位主入口。
- `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md`：Pages 质量门禁、同步与 SEO 约束主入口。
- `doc/site/manual/site-manual-static-docs.prd.md`：静态 docs hub 与手册镜像策略主入口。
- `doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md`：Viewer 手册镜像内容迁移与 canonical/manual 映射补充入口。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫全部 github-pages 与 manual 专题表。
- 完整活跃专题清单继续保留在下方，用于精确文件名检索和互链可达性。
- 公开 HTML 页面、同步脚本和镜像页继续保留可检索性，但默认不与专题三件套同屏平铺成长名单。

## 覆盖规则
- 纳入规则：纳入 `doc/site/{github-pages,manual}/*.prd.md` 与同名 `*.design.md` / `*.project.md` 的活跃专题三件套。
- 活跃补充：仍承担当前公开边界判断职责的高频专题，可在“活跃补充文档”区定向列出，但不再替代完整清单。
- 排除规则：`site/**` 下的公开 HTML 页面、同步脚本与镜像产物不并入专题三件套长表，只在补充入口中定向说明。
- 按需进入：当 `README.md` 与 `project.md` 已能完成首读分流时，本页只承担精确检索与补充路由职责。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/site/github-pages/github-pages-architecture-svg-refresh.prd.md` | `doc/site/github-pages/github-pages-architecture-svg-refresh.design.md` | `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md` |
| `doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md` | `doc/site/github-pages/github-pages-benchmark-polish-v3.design.md` | `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md` |
| `doc/site/github-pages/github-pages-content-sync-2026-02-12.prd.md` | `doc/site/github-pages/github-pages-content-sync-2026-02-12.design.md` | `doc/site/github-pages/github-pages-content-sync-2026-02-12.project.md` |
| `doc/site/github-pages/github-pages-content-sync-2026-02-25.prd.md` | `doc/site/github-pages/github-pages-content-sync-2026-02-25.design.md` | `doc/site/github-pages/github-pages-content-sync-2026-02-25.project.md` |
| `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` | `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.design.md` | `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md` |
| `doc/site/github-pages/github-pages-game-first-home-2026-02-25.prd.md` | `doc/site/github-pages/github-pages-game-first-home-2026-02-25.design.md` | `doc/site/github-pages/github-pages-game-first-home-2026-02-25.project.md` |
| `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.design.md` | `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-hero-motion-layer.prd.md` | `doc/site/github-pages/github-pages-hero-motion-layer.design.md` | `doc/site/github-pages/github-pages-hero-motion-layer.project.md` |
| `doc/site/github-pages/github-pages-hero-pointer-interaction.prd.md` | `doc/site/github-pages/github-pages-hero-pointer-interaction.design.md` | `doc/site/github-pages/github-pages-hero-pointer-interaction.project.md` |
| `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.design.md` | `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.design.md` | `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-lean-tech-refresh.prd.md` | `doc/site/github-pages/github-pages-lean-tech-refresh.design.md` | `doc/site/github-pages/github-pages-lean-tech-refresh.project.md` |
| `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.design.md` | `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.prd.md` | `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.design.md` | `doc/site/github-pages/github-pages-release-communication-placeholder-2026-03-11.project.md` |
| `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md` | `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.design.md` | `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md` |
| `doc/site/github-pages/github-pages-showcase.prd.md` | `doc/site/github-pages/github-pages-showcase.design.md` | `doc/site/github-pages/github-pages-showcase.project.md` |
| `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.design.md` | `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.prd.md` | `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.design.md` | `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.project.md` |
| `doc/site/manual/site-manual-static-docs.prd.md` | `doc/site/manual/site-manual-static-docs.design.md` | `doc/site/manual/site-manual-static-docs.project.md` |
| `doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md` | `doc/site/manual/viewer-manual-content-migration-2026-02-15.design.md` | `doc/site/manual/viewer-manual-content-migration-2026-02-15.project.md` |

## 公开镜像 / 手册补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `site/doc/cn/index.html` | `public_html` | 中文 docs hub 公开入口 |
| `site/doc/en/index.html` | `public_html` | 英文 docs hub 公开入口 |
| `site/doc/cn/viewer-manual.html` | `public_html` | 中文 Viewer 手册只读镜像 |
| `site/doc/en/viewer-manual.html` | `public_html` | 英文 Viewer 手册只读镜像 |
| `site/index.html` | `public_html` | 中文公开首页 |
| `site/en/index.html` | `public_html` | 英文公开首页 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- ROUND-002 口径：`doc/site/manual/site-manual-static-docs.prd.md` 为 manual 主文档，`doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md` 为增量子文档。
- ROUND-002 口径：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` 为 github-pages 主文档，其余 github-pages 专题为增量子文档。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法完成分流时，才进入后续路径级治理。
