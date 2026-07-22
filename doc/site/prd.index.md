# site PRD 文件级索引

审计轮次: 7

更新时间：2026-07-08

## 入口
- 模块 PRD：`doc/site/prd.md`
- 模块设计总览：`doc/site/design.md`
- 模块标准执行入口：`doc/site/project.md`
- 当前高频 site 入口：`doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`

## 首读分流
- 想先回答 site 模块在管什么、哪些公开边界是当前真值：先读 `doc/site/prd.md`
- 想先回答当前站点同步状态、最近完成项和是否还有未收口任务：先读 `doc/site/project.md`
- 想先看公开 docs hub 与手册镜像的 canonical 策略：先读 `doc/site/manual/README.md`
- 想先看 GitHub Pages 下载链路与公开版本说明边界：先读 `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`
- 想先看“正式公告仍在准备中”的当前公开口径：先读 `doc/site/prd.md`、`doc/site/project.md`，再对照 `site/index.html` 与 `site/doc/cn/index.html`
- 想继续按子域或文件名下钻：使用下方密度快照、热点子域导航与补充入口

## 密度快照（2026-07-08）
- `doc/site/`：64 份文件
- `doc/site/github-pages/`：50 份文件
- `doc/site/manual/`：7 份文件（含本地首读入口）
- 模块根入口：5 份文件
- `doc/site/` 正式专题三件套：54 份文件
- 当前默认活跃专题三件套：54 份文件
- 历史压缩专题三件套：0 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `github-pages/` 当前默认活跃专题三件套 | 48 | 公开首页、下载链路、内容同步、SEO 与质量门禁 |
| `github-pages/` 已退役删除旧专题 | 2 个专题 | 完成态公告占位与 CTA 微专题已删除；只从当前入口、GitHub task issue evidence comments 与 git history 追溯 |
| `manual/` 路由入口与正式专题三件套 | 7 | 静态 docs hub、Viewer 手册镜像与 canonical/manual 映射 |
| 模块根入口 | 5 | 模块目标态、执行台账、设计总览与文件级精确检索 |

## 活跃补充文档
- `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`：GitHub Pages 下载链路、发布资产和公开版本说明主入口。
- `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`：站点主叙事、docs hub 与 game-first 入口重定位主入口。
- `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md`：Pages 质量门禁、同步与 SEO 约束主入口。
- `doc/site/manual/README.md`：静态 docs hub、Viewer 手册镜像与 canonical/manual 映射的首读入口；再分流到主专题或已完成增量记录。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫全部 github-pages 与 manual 专题表。
- 当前默认活跃专题清单继续保留在下方，用于精确文件名检索和互链可达性。
- 已完成且只承担历史证据职责、且当前入口已承接的微专题优先进入“已退役删除的旧专题”，不再保留默认文档面文件。
- 公开 HTML 页面、同步脚本和镜像页继续保留可检索性，但默认不与专题三件套同屏平铺成长名单。

## 覆盖规则
- 纳入规则：纳入 `doc/site/{github-pages,manual}/*.prd.md` 与同名 `*.design.md` / `*.project.md` 的当前默认活跃专题三件套。
- 活跃补充：仍承担当前公开边界判断职责的高频专题，可在“活跃补充文档”区定向列出，但不再替代完整清单。
- 历史压缩：已完成、无下一步、由模块 project 或后续主专题覆盖当前真值的专题，可先从默认活跃清单降级；当当前入口、task evidence 与 git history 足以追溯时，继续删除旧专题文件并转入“已退役删除的旧专题”。
- 排除规则：`site/**` 下的公开 HTML 页面、同步脚本与镜像产物不并入专题三件套长表，只在补充入口中定向说明。
- 按需进入：当 `README.md` 与 `project.md` 已能完成首读分流时，本页只承担精确检索与补充路由职责。

## 当前默认活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/site/github-pages/github-pages-architecture-svg-refresh.prd.md` | `doc/site/github-pages/github-pages-architecture-svg-refresh.design.md` | `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md` |
| `doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md` | `doc/site/github-pages/github-pages-benchmark-polish-v3.design.md` | `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md` |
| `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` | `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.design.md` | `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md` |
| `doc/site/github-pages/github-pages-hero-motion-layer.prd.md` | `doc/site/github-pages/github-pages-hero-motion-layer.design.md` | `doc/site/github-pages/github-pages-hero-motion-layer.project.md` |
| `doc/site/github-pages/github-pages-hero-pointer-interaction.prd.md` | `doc/site/github-pages/github-pages-hero-pointer-interaction.design.md` | `doc/site/github-pages/github-pages-hero-pointer-interaction.project.md` |
| `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.design.md` | `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.design.md` | `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-lean-tech-refresh.prd.md` | `doc/site/github-pages/github-pages-lean-tech-refresh.design.md` | `doc/site/github-pages/github-pages-lean-tech-refresh.project.md` |
| `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.design.md` | `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md` | `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.design.md` | `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md` |
| `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.prd.md` | `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.design.md` | `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.project.md` |
| `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.prd.md` | `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.design.md` | `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.project.md` |
| `doc/site/manual/site-manual-static-docs.prd.md` | `doc/site/manual/site-manual-static-docs.design.md` | `doc/site/manual/site-manual-static-docs.project.md` |

## 历史压缩专题清单
当前无保留原址的历史压缩专题；已完成且有当前承接面的旧专题已转入下方退役删除清单。

## 公开镜像 / 手册补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `site/doc/cn/index.html` | `public_html` | 中文 docs hub 公开入口 |
| `site/doc/en/index.html` | `public_html` | 英文 docs hub 公开入口 |
| `site/doc/cn/viewer-manual.html` | `public_html` | 中文 Viewer 手册只读镜像 |
| `site/doc/en/viewer-manual.html` | `public_html` | 英文 Viewer 手册只读镜像 |
| `site/index.html` | `public_html` | 中文公开首页 |
| `site/en/index.html` | `public_html` | 英文公开首页 |

## 已退役删除的旧专题
| 旧专题 | 当前入口 |
| --- | --- |
| `github-pages-hero-cta-simplify-2026-02-26` | 该完成态 CTA 微专题三件套已删除；当前首页叙事、CTA 与下载入口真值以 `doc/site/project.md`、`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`、`doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md`、`site/index.html` 与 `site/en/index.html` 为准，历史细节从 GitHub task issue evidence comments 与 git history 追溯。 |
| `github-pages-release-communication-placeholder-2026-03-11` | 该完成态公告占位三件套已删除；当前公开公告准备态与 technical preview 边界以 `doc/site/prd.md`、`doc/site/project.md`、`site/index.html` 与 `site/doc/cn/index.html` 为准，历史细节从 GitHub task issue evidence comments 与 git history 追溯。 |
| `viewer-manual-content-migration-2026-02-15` | 该完成态 Viewer 手册搬迁三件套已删除；玩家模式承诺以 `doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md` 为准，当前操作及退役边界以 `doc/world-simulator/viewer/viewer-manual.manual.md` 为准，CN/EN 镜像治理以 `doc/site/manual/site-manual-static-docs.*` 为准，任务过程从 GitHub task issue evidence comments 与 git history 追溯。 |
| `github-pages-game-first-home-2026-02-25` | 该完成态首页游戏优先重排三件套已删除；当前公开首页语义与状态边界以 `doc/site/prd.md`、`doc/site/project.md`、`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`、`site/index.html` 与 `site/en/index.html` 为准，任务过程以 GitHub task #2515 evidence comments 与 git history 追溯。 |
| `github-pages-showcase` | 该完成态首版对外展示站三件套已删除；当前公开站点结构、双语入口与状态边界以 `doc/site/prd.md`、`doc/site/project.md`、`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`、`site/index.html` 与 `site/en/index.html` 为准，任务过程以 GitHub task #2515 evidence comments 与 git history 追溯。 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- ROUND-002 口径：`doc/site/manual/site-manual-static-docs.prd.md` 为 manual 主文档；已完成的 Viewer 手册搬迁增量已回填并退役，默认阅读先经 `doc/site/manual/README.md` 分流。
- ROUND-002 口径：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` 为 github-pages 主文档，其余 github-pages 专题为增量子文档。
- GitHub Pages content-sync historical/provenance note：`doc/site/github-pages/github-pages-content-sync-2026-02-12.*` 与 `doc/site/github-pages/github-pages-content-sync-2026-02-25.*` 保留为已完成内容同步批次的来源线索，不再作为当前默认活跃专题清单入口；当前 site 真值以 `doc/site/prd.md`、`doc/site/project.md` 为准，github-pages 主文档以 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` 为准，下载/公开面按 `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md` 与 `site/**` 现行文件追溯。后续若删除这些历史子文档，需先完成引用审计与 deletion-readiness slice。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法完成分流时，才进入后续路径级治理。
