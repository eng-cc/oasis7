# GitHub Pages 质量门禁 + 文档镜像同步 + SEO 元信息加固（2026-02-26）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md`

审计轮次: 5

- 对应标准执行入口: `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md`

## ROUND-002 主从口径
- 主入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`
- 本文仅维护本专题增量，不重复主文档口径。

## 目标
- 建立 GitHub Pages 发布前的最小质量门禁，避免“改了就发”导致线上回归。
- 消除 `doc/world-simulator/viewer/viewer-manual.manual.md` 与 `site/doc/cn|en/viewer-manual.html` 的关键命令路径漂移。
- 补齐首页 SEO/社交元信息，提升外部分享与搜索展示稳定性。

## 范围
- 范围内
  - 修复 `site/doc/cn/viewer-manual.html` 与 `site/doc/en/viewer-manual.html` 中已过时的 agent-browser 路径口径。
  - 新增站点校验脚本（链接可达、镜像关键口径一致）。
  - 将校验脚本接入 `.github/workflows/pages.yml`，作为部署前门禁。
  - 更新 `site/index.html` 与 `site/en/index.html` 的 SEO/社交元信息（`og:url`、`twitter:title/description/image`、`theme-color`，并统一 `og:image` 绝对 URL）。
- 范围外
  - 不引入新的前端构建系统或 SSG。
  - 不重写 `site/` 视觉布局与交互行为。
  - 不改 Rust 运行时逻辑。

## 接口/数据
- 校验脚本接口：
  - `scripts/site-link-check.sh`：校验 `site/*.html` 与 `site/doc/**/*.html` 的本地相对链接存在性。
- `scripts/site-manual-sync-check.sh`：校验站内手册镜像与主手册关键路径口径一致（统一校验 `agent-browser`，并保留仓库开发副本 `.agents/...` 说明）。
- CI 接口：
  - `.github/workflows/pages.yml` 在 `Upload Pages artifact` 之前执行上述校验。
- SEO 字段：
  - `site/index.html`、`site/en/index.html` 增加/修正 `og:url`、`og:image`（绝对 URL）与 Twitter 相关字段。

## 里程碑
- M0：建档与任务拆解。
- M1：修复站内手册镜像路径口径并新增镜像一致性检查脚本。
- M2：新增链接检查并接入 Pages 工作流部署门禁。
- M3：完成首页 SEO 元信息补齐与绝对 URL 统一。
- M4：完成验证、项目文档回写、devlog 记录与提交收口。

## 风险
- 风险：手册正文长期仍可能漂移。
  - 缓解：把关键口径校验纳入 Pages 部署门禁，阻断明显过期内容上线。
- 风险：链接检查脚本误报导致发布受阻。
  - 缓解：仅检查站内相对路径，跳过外链与锚点；脚本输出失败明细便于快速修复。
- 风险：SEO 字段修改影响现有分享卡片。
  - 缓解：采用向后兼容字段补充，不删除原有 canonical/hreflang 配置。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
