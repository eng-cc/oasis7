# site PRD Project

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-SITE-001 (PRD-SITE-001) [test_tier_required]: 完成 site PRD 改写，建立站点设计主入口。
- [x] TASK-SITE-002 (PRD-SITE-001/002) [test_tier_required]: 固化站点信息架构与内容同步校验清单。
- [x] TASK-SITE-003 (PRD-SITE-002/003) [test_tier_required]: 补齐发布下载链路与SEO质量门禁说明。
- [x] TASK-SITE-004 (PRD-SITE-003) [test_tier_required]: 建立站点发布后质量回归节奏。
- [x] TASK-SITE-005 (PRD-SITE-001/002/003) [test_tier_required]: 对齐 strict PRD schema，补齐关键流程/规格矩阵/边界异常/NFR/验证与决策记录。
- [x] TASK-SITE-006 (PRD-SITE-002) [test_tier_required]: 同步 `site/doc/cn|en/viewer-manual.html` 语义口径（移除过时 `power_storage`，并校准自动目标语法）。
- [x] TASK-SITE-007 (PRD-SITE-003) [test_tier_required]: 回写站点项目状态文档（release pipeline + module 主项目）并与 CI 实况对齐。
- [x] TASK-SITE-008 (PRD-SITE-004) [test_tier_required]: 修复首页与文档入口“可玩状态”口径，明确当前为开发中技术预览（尚不可玩）。
- [x] TASK-SITE-009 (PRD-SITE-001/004) [test_tier_required]: 在保持真实状态口径前提下重排 CTA 与信息层级（预览体验优先、构建路径次级）。
- [x] TASK-SITE-010 (PRD-SITE-005/006/007) [test_tier_required]: 在公开首页与文档入口补齐“正式公告仍在准备中”的安全占位，并区分构建说明与正式公告。
- [x] TASK-SITE-011 (PRD-SITE-003) [test_tier_required]: 同步 `doc/site/README.md` 目录索引，补齐最新 github-pages 专题入口。
  - 产物文件:
    - `site/index.html`
    - `site/en/index.html`
    - `site/doc/cn/index.html`
    - `site/doc/en/index.html`
    - `doc/site/github-pages/viewer-to-producer-task-site-009-cta-priority-2026-03-11.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "先看技术预览路径|See Preview Verification Path|优先级：预览体验入口优先|Priority: preview experience first" site/index.html site/en/index.html site/doc/cn/index.html site/doc/en/index.html`
    - `./scripts/doc-governance-check.sh`
- [x] TASK-SITE-012 (PRD-SITE-003) [test_tier_required]: 为 completed 状态的 `doc/site/project.md` 补齐“最新完成”摘要，保持模块项目状态栏格式一致。
- [x] TASK-SITE-013 (PRD-SITE-007) [test_tier_required]: 同步中英首页的预览访问面口径，明确 `standard_3d / software_safe / pure_api` 是当前技术验证访问面，`LLM/script` 与 OpenClaw lane 只属于执行方式而非额外公开模式。
- [x] TASK-SITE-014 (PRD-SITE-003) [test_tier_required]: 修复 `scripts/site-manual-sync-check.sh` 的 viewer manual 基线命令，追平 `test_api=1` 新入口，解除 Pages `Site Quality Gates` 的假失败。
- [x] TASK-SITE-015 (PRD-SITE-008) [test_tier_required]: 统一 `site/**`、GitHub Release 下载入口、站点检查脚本与 release workflow 的公开品牌为 `oasis7`，并同步 `eng-cc/oasis7` 仓库路径与 `oasis7-*` 资产名。
- [x] TASK-SITE-016 (PRD-SITE-008) [test_tier_required]: 对 `doc/site/github-pages/**` 的仍可读历史专题执行 title-only cleanup，将首行 `oasis7*` 公开标题统一切到 `oasis7*`，保留正文历史证据原文不动。
- [x] TASK-SITE-017 (PRD-SITE-008) [test_tier_required]: 将 `release-packages.yml` 的 Web dist / soak prewarm 当前路径与包名切到 `oasis7*`，并同步 `github-pages-release-download-pipeline` 当前真值文档。
  - 产物文件:
    - `doc/site/prd.md`
    - `doc/site/project.md`
    - `doc/site/github-pages/*.md`
    - `doc/devlog/2026-03-19.md`
  - 验收命令 (`test_tier_required`):
    - `rg -n "^# oasis7|^# oasis7 Runtime|^# oasis7 Simulator|^# oasis7 Viewer" doc/site --glob '!third_party/**'`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-SITE-018 (PRD-SITE-008) [test_tier_required]: 收口 `doc/site/github-pages/**` 活跃专题中的当前 viewer 校验命令与 crate 路径，统一使用 `oasis7_viewer` / `crates/oasis7*` 口径。
  - 验收命令 (`test_tier_required`):
    - `rg -n "oasis7_viewer|crates/oasis7/src/bin/oasis7_viewer_live.rs|crates/oasis7_viewer/src/" doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md doc/site/github-pages/github-pages-game-first-home-2026-02-25.project.md doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.project.md doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.project.md doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.project.md doc/site/github-pages/github-pages-content-sync-2026-02-25.prd.md doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md`
    - `./scripts/doc-governance-check.sh`
    - `git diff --check`
- [x] TASK-SITE-019 (PRD-SITE-002/003) [test_tier_required]: 执行 ROUND-009 `site` 模块入口映射治理，明确 `doc/site/README.md`、公开 docs hub、手册镜像策略与仓库权威手册的职责边界。
- [x] TASK-SITE-020 (PRD-SITE-002/003) [test_tier_required]: 同步 GitHub Pages Viewer 手册镜像与 docs hub 口径到当前 canonical LLM gating，并修正 Pages / sync gate 对 canonical 手册路径的监听。

## 依赖
- 模块设计总览：`doc/site/design.md`
- doc/site/prd.index.md
- `site/`
- `site/doc/`
- `doc/site/github-pages/`
- `doc/site/manual/`
- `doc/readme/prd.md`
- `.agents/skills/prd/check.md`

## 状态
- 更新日期: 2026-04-02
- 当前状态: completed
- 下一任务: 无（当前模块主项目无未完成任务）
- 最新完成: `TASK-SITE-020`（`site/doc/cn|en/viewer-manual.html` 已追平 current LLM gating 口径，docs hub 已移除陈旧同步日期，Pages workflow 与 `site-manual-sync-check` 已改为监听 canonical `viewer-manual.manual.md`。）
- 最新完成: `TASK-SITE-019`（已为 `doc/site/README.md` 补齐 site 模块与公开 docs hub、静态手册镜像、仓库权威手册之间的入口映射。）
- 最新完成: `TASK-SITE-018`（`doc/site/github-pages/**` 活跃专题中的当前 viewer 校验命令与 crate 路径已统一切到 `oasis7_viewer` / `crates/oasis7*` 当前口径。）
- 最新完成: `TASK-SITE-017`（`release-packages.yml` 的 Web dist / soak prewarm 当前路径与包名已切到 `oasis7*`，相关 github-pages 发布链路文档已同步。）
- 最新完成: `TASK-SITE-015`（公开站点、release 下载入口与站点脚本已统一切换到 `oasis7` 品牌与 `eng-cc/oasis7` 路径）。
- 最新完成: `TASK-SITE-016`（已完成 `doc/site/github-pages/**` 历史专题首行标题的 title-only cleanup，旧 `oasis7*` 公开标题已统一切到 `oasis7*`）。
- 最新完成: `TASK-SITE-011`（site 模块 README 目录索引同步）。
- 最新完成: `TASK-SITE-012`（site 模块 completed 状态摘要补齐）。
- 最新完成: `TASK-SITE-013`（中英首页已对齐三模式技术预览访问面口径，消除旧“运行模式/LLM 默认模式”歧义）。
- 最新完成: `TASK-SITE-014`（site manual sync gate 已追平 `test_api=1` viewer 命令基线，解除 Pages 假失败）。
- PRD 质量门状态: strict schema 已对齐（含第 6 章验证与决策记录）。
- ROUND-002 进展: manual 子簇主从化已完成（`site-manual-static-docs` 主入口，`viewer-manual-content-migration-2026-02-15` 增量维护）。
- ROUND-002 进展: github-pages 子簇主从化已完成（`github-pages-game-engine-reposition-2026-02-25` 主入口，其余专题增量维护）。
- ROUND-003 进展: TASK-SITE-002/003/004 已按既有专题交付收敛为 completed，进入口径同步与状态回写阶段（TASK-SITE-006/007）。
- ROUND-003 进展: TASK-SITE-006 已完成，静态手册已移除过时 `power_storage` 表述并校准自动目标语法。
- ROUND-003 进展: TASK-SITE-007 已完成，`github-pages-release-download-pipeline-2026-03-01.project.md` 状态已与 `Release Packages` 最新成功 run 对齐。
- ROUND-004 进展: 已识别首页“已可玩/赛季运行中”叙事与真实状态不一致，进入 TASK-SITE-008 修复口径。
- ROUND-004 进展: TASK-SITE-008 已完成；`site/index.html`、`site/en/index.html`、`site/doc/cn/index.html`、`site/doc/en/index.html` 已统一为“技术预览（尚不可玩）”口径。
- ROUND-005 进展: TASK-SITE-009 已完成；首页与文档入口 CTA 已调整为“预览验证优先、完整构建/长文档次级”的同构层级。
- ROUND-005 进展: TASK-SITE-010 已完成；首页/下载区与 docs hub 已补“正式公告仍在准备中”的公开说明占位，并明确 release notes 仅代表构建说明。
- ROUND-005 进展: TASK-SITE-011 已完成；`doc/site/README.md` 已同步最新 github-pages 专题入口与维护约定。
- 说明: 本文档仅维护 site 模块设计执行状态；过程记录在 `doc/devlog/2026-03-03.md` 与 `doc/devlog/2026-03-11.md`。
