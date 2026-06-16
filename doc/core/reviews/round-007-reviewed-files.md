# ROUND-007 逐文档内容职责边界复核清单（Compact Snapshot）

审计轮次: 7

## 清单状态
- 当前形式: compact historical snapshot entrypoint
- 统计口径: `doc/**/*.md` 排除 `doc/devlog/**`，即 ROUND-007 固定分母
- 当前目标范围文档数: 874
- 当前已完成复核文档数: 874
- 当前状态: `completed`
- 全量逐行证据: 保存在 pre-compaction git snapshot `0d6fd50849cae07bac17883cca14f141ede93196`

## 全量逐行证据恢复
当前文件保留 ROUND-007 的固定分母、完成态、字段契约与模块分布，避免 active `doc/` 长表持续逼近行数门禁。

恢复完整 874 行逐文档清单:

```bash
git show 0d6fd50849cae07bac17883cca14f141ede93196:doc/core/reviews/round-007-reviewed-files.md
```

验证历史完成态:

```bash
./scripts/doc-evidence-snapshot-check.sh
```

## 字段说明
| 字段 | 说明 |
| --- | --- |
| 文档路径 | 复核文档的仓库相对路径 |
| 当前类型 | 当前文档类型：`prd` / `design` / `project` / `manual` / `runbook` / `readme` / `index` / `legacy_misc` |
| 边界判定 | `pass` / `mixed_prd_design` / `authority_drift` / `manual_overreach` / `待判定` |
| 主要问题编号 | 主要对应 `I7-*`；无问题写 `none` |
| 整改动作 | `keep` / `trim` / `retarget` / `split` / `backfill_links` |
| 索引回写 | `pending` / `done` / `n/a` |
| 引用回写 | `pending` / `done` / `n/a` |
| owner role | 默认牵头角色 |
| 状态 | `pending` / `in_progress` / `done` / `blocked` |
| 备注 | 补充说明 |

## 汇总
| 范围 | 文档数 | 状态 |
| --- | --- | --- |
| ROUND-007 总范围（`doc/**/*.md` - `doc/devlog/**`） | 874 | completed |
| 合计 | 874 | completed |

## 模块分布（按顶层目录）
| 模块 | 文档数 |
| --- | --- |
| `README.md` | 1 |
| `core` | 32 |
| `engineering` | 30 |
| `game` | 33 |
| `game-test.prd.md` | 1 |
| `game-test.project.md` | 1 |
| `headless-runtime` | 13 |
| `historical removed viewer docs` | 80 |
| `p2p` | 156 |
| `playability_test_card.md` | 1 |
| `playability_test_manual.md` | 1 |
| `playability_test_result` | 16 |
| `readme` | 35 |
| `scripts` | 11 |
| `site` | 43 |
| `testing` | 63 |
| `ui_review_result` | 3 |
| `viewer-manual.md` | 1 |
| `world-runtime` | 71 |
| `world-runtime.prd.md` | 1 |
| `world-runtime.project.md` | 1 |
| `world-simulator` | 278 |
| `world-simulator.prd.md` | 1 |
| `world-simulator.project.md` | 1 |

## 当前入口校验
```bash
./scripts/doc-evidence-snapshot-check.sh
```
