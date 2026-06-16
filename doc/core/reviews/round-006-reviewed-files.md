# ROUND-006 逐文档结构治理清单（Compact Snapshot）

审计轮次: 6

## 清单状态
- 生成时间: 2026-03-09（全量清单初始化）
- 当前形式: compact historical snapshot entrypoint
- 统计口径: `doc/**/*.md` 排除 `doc/devlog/**`，即 ROUND-006 全量治理分母
- 当前目标范围文档数: 870
- 当前已完成治理文档数: 870
- 当前状态: `completed`
- 全量逐行证据: 保存在 pre-compaction git snapshot `0d6fd50849cae07bac17883cca14f141ede93196`

## 全量逐行证据恢复
当前文件保留 ROUND-006 的固定分母、完成态、字段契约与模块分布，避免 active `doc/` 长表持续逼近行数门禁。

恢复 870 份文档分母对应的历史清单:

```bash
git show 0d6fd50849cae07bac17883cca14f141ede93196:doc/core/reviews/round-006-reviewed-files.md
```

验证历史逐行证据仍保留完成态与明细章节:

```bash
./scripts/doc-evidence-snapshot-check.sh
```

## 字段说明
| 字段 | 含义 |
| --- | --- |
| 文档路径 | 治理文档的仓库相对路径 |
| 当前类型 | 当前文档实际承担的职责类型 |
| 目标类型 | 按规范应落位的目标类型 |
| 是否需重命名 | 是否需要 rename |
| 是否需拆分/合并 | 是否需要 split / merge |
| design 缺口 | 是否需要补 `*.design.md` 或模块 `design.md` |
| 索引回写 | `pending` / `done` / `n/a` |
| 引用回写 | `pending` / `done` / `n/a` |
| 改造动作 | `rename` / `split` / `merge` / `backfill` / `retarget` / `keep` / `pending_scan` |
| owner role | 默认牵头角色 |
| 状态 | `todo` / `doing` / `done` / `blocked` |
| 备注 | 补充说明 |

## 总范围与批次
| 口径 | 文档数 | 状态 |
| --- | --- | --- |
| ROUND-006 总范围（`doc/**/*.md` - `doc/devlog/**`） | 870 | completed |
| B6-001 模块入口治理 | 49 | done |
| B6-002 专题三件套治理 | 333 | done |
| B6-003 索引/互链治理 | 完成并入 B6-001/B6-002 | done |
| 合计 | 870 | completed |

## 模块分布（按顶层目录）
| 模块 | 文档数 |
| --- | --- |
| `(root)` | 10 |
| `core` | 29 |
| `engineering` | 31 |
| `game` | 34 |
| `headless-runtime` | 14 |
| `p2p` | 157 |
| `playability_test_result` | 17 |
| `readme` | 36 |
| `scripts` | 24 |
| `site` | 44 |
| `testing` | 64 |
| `ui_review_result` | 3 |
| `world-runtime` | 72 |
| `world-simulator` | 347 |

## 当前入口校验
```bash
./scripts/doc-evidence-snapshot-check.sh
```
