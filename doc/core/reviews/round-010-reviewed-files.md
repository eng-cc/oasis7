# ROUND-010 延期模块入口分流清单

审计轮次: 10

## 清单状态
- 当前 focused scope 数: 6
- 当前已完成首轮判定对象数: 6
- 当前状态: `in_progress`

## 字段说明
| 字段 | 说明 |
| --- | --- |
| 文档路径 | 被纳入 ROUND-010 的 focused scope 对象 |
| 当前角色 | 当前承担的消费/治理角色 |
| 关注点 | 本轮主要判断的问题 |
| 建议动作 | `keep` / `split` / `defer` |
| 优先级 | `P0` / `P1` / `P2` |
| owner role | 牵头角色 |
| 当前状态 | `scoped` / `aligned` / `deferred` |
| 问题编号 | 对应 `I10-*` |
| 备注 | 当前已知事实或后续触发条件 |

## 汇总
| 范围 | 数量 | 状态 |
| --- | --- | --- |
| focused scope 文档总数 | 6 | in_progress |

## 明细
| 文档路径 | 当前角色 | 关注点 | 建议动作 | 优先级 | owner role | 当前状态 | 问题编号 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `doc/world-runtime/README.md` | 高体量 runtime 模块入口 | 是否需要补“先读哪里”与长表索引边界 | split | P0 | `producer_system_designer` | aligned | I10-001 | 已补“从这里开始”与入口分工，明确 README / `prd.index.md` / 高频 runtime/wasm/module 专题的阅读边界 |
| `doc/p2p/README.md` | 高体量网络模块入口 | 是否需要补任务导向起点与高频主题分流 | split | P1 | `producer_system_designer` | aligned | I10-001 | 已补任务导向入口，并明确 README / `prd.index.md` / 主链安全 / hosted world / token-governance signer 高频专题的阅读边界 |
| `doc/scripts/README.md` | 工具模块入口 | 是否要补读者角色与“先用哪个脚本入口” | keep | P1 | `producer_system_designer` | aligned | I10-002 | 已补轻量入口映射，明确 README / `prd.index.md` / task-worktree bootstrap / landing / harness 的阅读边界 |
| `doc/game/README.md` | 玩法模块入口 | 是否需要补产品目标 / 玩法 / 发布口径的阅读顺序 | keep | P1 | `producer_system_designer` | scoped | I10-003 | 当前近期专题较密集，需确认是否有必要再加入口层 |
| `doc/playability_test_result/README.md` | 证据模块入口 | 是否需明确“证据使用者优先”而非新读者 landing | keep | P1 | `qa_engineer` | scoped | I10-003 | 先确认是否维持 QA/追溯导向即可 |
| `doc/headless-runtime/README.md` | 基础设施模块入口 | 是否需补命名迁移后的读者说明与使用前提 | keep | P2 | `producer_system_designer` | scoped | I10-002 | 当前已含 rename 说明，优先级低于 runtime/p2p/scripts |
