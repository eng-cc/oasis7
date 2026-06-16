# ROUND-006 结构治理执行清单

审计轮次: 6

## 目标
- 将 ROUND-006 定义为“按 `doc-structure-standard` 逐文档改造”的结构治理轮，而不是单纯审计轮。
- 形成“固定总范围 -> 分批逐文档改造 -> 回写入口/索引/引用 -> 复审验收”的闭环。

## 执行阶段
| 阶段 | 动作 | 产物 | 状态 |
| --- | --- | --- | --- |
| P0 | 建立 ROUND-006 执行台账骨架 | `consistency-review-round-006.md`、`round-006-reviewed-files.md`、`round-006-kickoff-worklist.md`、`round-006-audit-progress-log.md` | done |
| P1 | 固定 ROUND-006 总范围与问题域，冻结 owner 与批次 | 总范围、`G6-*`、`I6-*` | done |
| P2 | 生成全量逐文档治理清单；当前保留 compact snapshot entrypoint，逐行证据由 pre-compaction git snapshot 恢复 | `round-006-reviewed-files.md` | done |
| P3 | 按批次执行 rename/split/merge/backfill/retarget | 文档改造提交 + 索引/引用回写 | done |
| P4 | 运行治理门禁与 ROUND-006 复审 | 快速结构校验 / 引用可达性校验 / 复审结论 | done |

## 并行批次（首版建议）
| 批次 | 范围 | 目标问题 | owner role | 状态 |
| --- | --- | --- | --- | --- |
| B6-001 | 模块入口文档（根入口 + 12 个模块 README/design/project/index） | D6-003/D6-004/D6-005 | `producer_system_designer` | done |
| B6-002 | 专题三件套与 basename（全仓 `.project.md` 收口） | D6-001/D6-002/D6-007 | 对应模块 owner | done |
| B6-003 | 索引、README、历史引用与 redirect | D6-004/D6-006 | 对应模块 owner | done |
| B6-004 | 复审与阻断结论 | 全量复核 | `qa_engineer` | done |

## 执行原则
- 总范围固定：ROUND-006 覆盖 `doc/**/*.md` 排除 `doc/devlog/**`，批次只影响顺序，不影响纳入范围。
- 不只登记问题：进入 P3 后，每条问题都必须落到具体文档改造动作。
- 先定权威源：若遇到规范冲突，先以 `doc-structure-standard` 为准；规范未定义时先补规范再改文档。
- 同批回写：任何 rename/split/merge 都必须同步回写 `README`、`prd.index.md`、`project.md` 与相关专题互链。
- 逐文档可追溯：总范围内每篇文档都要在治理清单中留下“当前类型 -> 目标类型 -> 改造动作 -> 状态”；当前入口为 compact snapshot，逐行证据使用 `git show 0d6fd50849cae07bac17883cca14f141ede93196:doc/core/reviews/round-006-reviewed-files.md` 恢复。
- 未完成不升轮次：未完成结构治理的文档不得提前回写 `审计轮次: 6`。

## 验收口径
- required：台账、清单、批次、字段定义齐全；文档改造动作可追溯。
- full：`doc-structure-standard` 范围内结构问题关闭，索引/互链/引用可达，治理门禁通过。

## 通用验收命令
- `test -f doc/core/reviews/consistency-review-round-006.md`
- `test -f doc/core/reviews/round-006-reviewed-files.md`
- `test -f doc/core/reviews/round-006-kickoff-worklist.md`
- `test -f doc/core/reviews/round-006-audit-progress-log.md`
- `rg -n "doc-structure-standard|当前类型|目标类型|改造动作|未完成不升轮次|compact snapshot" doc/core/reviews/consistency-review-round-006.md doc/core/reviews/round-006-reviewed-files.md doc/core/reviews/round-006-kickoff-worklist.md`
- `./scripts/doc-evidence-snapshot-check.sh`
