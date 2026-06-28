# ROUND-001 归档迁移执行清单（A-012）

审计轮次: 4

## 目标与策略
- 目标：对 `R-001~R-009` 给出“保留文档 + 替代链 + 索引回写 + redirect”执行清单，并实施首批迁移。
- redirect 策略（本轮）：采用 soft redirect（原文保留，文档头新增“历史状态 + 当前替代入口”），避免历史链接断链。
- 索引策略：从模块 `prd.index.md` 的活跃分组移出，统一放入“历史专题”分组。

## 首批迁移范围（已执行）
- `R-003`：`world-simulator/launcher` 旧 `desktop/unified` 专题。
- `R-004`：`world-simulator/viewer` 行数收口历史专题。

## 执行清单
| 编号 | 保留文档（源） | 替代链 | 索引回写 | redirect 方式 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R-001 | `doc/headless-runtime/nonviewer/nonviewer-design-alignment-*.prd*.md` | `doc/headless-runtime/prd.md` + ROUND 记录 | 待执行（headless-runtime 索引） | 计划 soft redirect | pending |
| R-002 | `doc/world-runtime/runtime/runtime-numeric-correctness-phase*` | `doc/world-runtime/prd.md` | 待执行（受 `S-001` 约束） | 计划 soft redirect | deferred (`S-001`) |
| R-003 | legacy launcher desktop/unified 文档（已删除） | `game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md` + `game-client-launcher-web-console-2026-03-04.prd.md` | 已执行：从索引中移除旧文档并保留替代入口 | 已执行：旧文档已删除 | done |
| R-004 | legacy viewer rust line cap 文档（已删除） | `viewer-release-full-coverage-gate.prd.md` + `viewer-visual-release-readiness-hardening-2026-03-01.prd.md` | 已执行：从索引中移除旧文档并保留替代入口 | 已执行：旧文档已删除 | done |
| R-005 | `doc/p2p/distributed/distributed-hard-split-phase7.prd*.md` 与 `doc/p2p/observer/observer-sync-*.prd*.md` 前序阶段 | `doc/p2p/distributed/distributed-runtime.prd.md` + 最新 observer 专题 | 待执行（p2p 索引分层） | 计划 soft redirect | pending |
| R-006 | `doc/p2p/*/*.release.md` | 对应 `*.prd.md` / `*.project.md` | 已执行：`doc/p2p/prd.index.md` 增加 release 可达分组 | 不做归档迁移（保留为补充材料） | kept_with_index |
| R-007 | historical PRD review checklist snapshots（后续已删除） | core review round logs + 文档头 `审计轮次` | 已执行：迁移为 historical snapshot prose，不再保留 checklist 目录活跃面 | 已执行：旧 checklist snapshots 已删除 | done |
| R-008 | `doc/site/github-pages/*.prd*.md`、`doc/site/manual/*.prd*.md` 的已收口日期专题 | `doc/site/prd.md` + `doc/site/project.md` | 待执行（site 索引治理） | 计划 soft redirect | pending |
| R-009 | `doc/readme/gap|production|governance` 与 `doc/game/gameplay` 已收口日期专题 | `doc/readme/prd.md`、`doc/game/prd.md`、`README.md` | 待执行（readme/game 索引治理） | 计划 soft redirect | pending |

## 验收命令（A-012）
- `test -f doc/core/reviews/round-001-archive-migration-plan.md`
- `rg -n "game-client-launcher-native-web-control-plane-unification-2026-03-04" doc/world-simulator/prd.index.md`
- `rg -n "viewer-release-full-coverage-gate" doc/world-simulator/prd.index.md`
- `./scripts/doc-governance-check.sh`
