# Core 文件命名语义审查记录（第003轮）

审计轮次: 4

## 目的
- 为 `TASK-CORE-005` 提供 ROUND-003 审查入口，聚焦“文件名是否语义化”。
- 本轮主线-1：检查 PRD/Project/Index 文件名是否能清晰表达“模块 + 主题 + 范围/阶段 + 时间”（必要时）。
- 本轮主线-2：对不语义化命名给出“更名方案 + 索引回写 + 引用替换”并执行。

## 轮次信息
- 轮次编号: `ROUND-003`
- 轮次状态: `completed` (`not_started` | `in_progress` | `completed`)
- 审查时间窗: 2026-03-05 ~ 2026-03-06
- 审查负责人: cc

状态判定：
- `not_started`: 仅完成启动文档/方法准备，尚未开始文档级审读与问题登记。
- `in_progress`: 已开始审读并登记命名问题/整改项，但未形成最终复审结论。
- `completed`: 命名问题已收口，整改项关闭或已登记批准延期备注。

## 文档级审计标记方法（缺省=0）
- 每个受审文档采用字段 `审计轮次: <整数>` 标识最新已完成审计轮次。
- 本轮执行规则：
  - 单篇文档完成 ROUND-003 审读后，在同一提交回写 `审计轮次: 3`（与是否需要更名解耦）。
  - 若判定“命名合格无需更名”，仍应回写 `审计轮次: 3`，并在本记录登记判定理由。
  - 若实施更名，需同步更新：`prd.index.md` 与引用路径（PRD/project/README/站点/脚本）。
  - 若尚未完成 ROUND-003 审读，则保持原值（缺失按 `0`）。
- 本轮完成条件：纳入 `S_round003` 的文档全部满足 `审计轮次 >= 3`，且复审结论已落档。

建议统计命令：
```bash
rg -n "^审计轮次:\s*3$" doc --glob '*.md'
```

## 文件命名语义判定规则
- 必须体现模块与主题：推荐 `module-topic-action` 或 `module-topic-scope` 结构。
- 时间敏感/阶段性专题必须包含日期（`YYYY-MM-DD`）或阶段号（`phaseN`），长期有效文档不强制日期。
- 避免模糊词：`misc`/`tmp`/`update`/`new`/`fix` 等不允许作为主语义。
- 允许 `phase/round` 仅在存在明确阶段序列时使用；避免“单篇文档带 phase1”但无后续阶段。
- PRD 配对规则：`*.prd.md` 与 `*.project.md` 必须一一对应且同名；更名需同步调整两者。
- 根入口/索引固定命名：`prd.md`、`project.md`、`prd.index.md` 不参与更名。

## 启动范围（命名风险分区）
- A: `doc/world-simulator/**/*`（日期专题密集，命名含历史阶段）
- B: `doc/p2p/**/*`（phase/round 语义与范围一致性）
- C: `doc/testing/*` + `doc/scripts/*`（测试与脚本专题命名规范化）
- D: `doc/site/*` + `doc/readme/*` + `doc/game/*`（外部曝光文档命名一致性）
- E: 其他模块 PRD 树（补查）

## 受审文件清单（S_round003）
- 清单文件：`doc/core/reviews/round-003-reviewed-files.md`
- 生成规则：`rg -l "^审计轮次:\s*3$" doc --glob '*.md' | sort`
- 当前基线（2026-03-05 23:50 CST）：`773` 份文档
- 用途：作为 ROUND-003 统计分母（仅对纳入本轮清单的文档判定“已审读/未审读”）。

## 命名问题清单
| 编号 | 问题描述 | 影响范围 | 建议命名 | 严重度 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I3-001 | viewer 子目录存在未带 `viewer-` 前缀的专题名，语义与同目录其它文档不一致。 | `doc/world-simulator/viewer/{minimal-system,module-visual-entities,rendering-physical-accuracy,visualization,visualization-3d}.prd(.project).md` | 已更名为 `viewer-*` 前缀版本。 | medium | `done` |
| I3-002 | launcher 文档命名前缀不一致：`launcher-` 与同目录 `game-*` 并存，语义入口不统一。 | `doc/world-simulator/launcher/launcher-chain-runtime-decouple-2026-02-28.prd(.project).md` | 已更名为 `game-client-launcher-chain-runtime-decouple-2026-02-28`. | low | `done` |
| I3-003 | distributed 子目录存在缺少 `distributed-` 前缀的 PRD，主题语义不直观。 | `doc/p2p/distributed/production-runtime-gap1234568-closure.prd(.project).md` | 已更名为 `distributed-production-runtime-gap1234568-closure`. | medium | `done` |
| I3-004 | node 子目录存在缺少 `node-` 前缀的 PRD，且 `readme-*` 语义不清晰。 | `doc/p2p/node/{builtin-wasm-fetch-fallback-compile,readme-p2-node-net-stack-unification}.prd(.project).md` | 已更名为 `node-builtin-wasm-fetch-fallback-compile` 与 `node-net-stack-unification-readme`. | medium | `done` |
| I3-005 | distfs 子目录存在缺少 `distfs-` 前缀的 PRD，命名与同目录不一致。 | `doc/p2p/distfs/{builtin-wasm-distfs-api-closure,builtin-wasm-distfs-storage}.prd(.project).md` | 已更名为 `distfs-builtin-wasm-api-closure`、`distfs-builtin-wasm-storage`. | medium | `done` |
| I3-006 | 工程专题使用 `round3` 但无其它轮次文件，阶段语义不成立。 | oversized Rust file splitting round3 docs (retired/deleted) | 当时已更名为无 `round3` slug；该三件套后续已退役删除，当前 Rust 1200 治理入口为 `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`。 | medium | `done` |
| I3-007 | headless-runtime 专题使用 `round2` 但无其它轮次文件，阶段语义不成立。 | nonviewer design-alignment review round2 docs (renamed, later retired/deleted) | 已更名为 `nonviewer-design-alignment-review-2026-02-25`. | medium | `done` |
| I3-008 | 命名以 `fix/ fixes` 作为主语义，语义模糊且难以长期复用。 | `doc/scripts/precommit/fix-precommit.prd(.project).md` | 已更名为 `precommit-remediation-playbook`. | low | `done` |
| I3-009 | viewer 多篇专题以 `*-fix` 结尾，语义偏“临时修复”。 | `doc/world-simulator/viewer/viewer-*-fix.prd(.project).md`（deadlock/frag-scale/2d-3d-clarity） | 已更名为 `*-resolution` / `*-stability` / `*-improvement` 风格命名。 | medium | `done` |
| I3-010 | nonviewer 设计对齐专题使用 `fixes`，语义模糊。 | nonviewer design-alignment fixes docs (renamed, later retired/deleted) | 已更名为 `nonviewer-design-alignment-closure-2026-02-25`. | low | `done` |
| I3-011 | github-pages 专题使用 `fixes`，命名偏过程词。 | `doc/site/github-pages/github-pages-user-perspective-fixes-2026-02-26.prd(.project).md` | 已更名为 `github-pages-user-perspective-adjustments-2026-02-26`. | low | `done` |

## 整改项
| 编号 | 整改动作 | 责任人 | 截止时间 | 状态 |
| --- | --- | --- | --- | --- |
| A3-001 | 建立 ROUND-003 启动台账（本文件 + 审读清单 + 执行清单） | cc | 2026-03-05 | done |
| A3-002 | 完成 A/B/C/D/E 分区命名审读并形成命名问题清单 | cc | 2026-03-08 | done |
| A3-003 | 对命名问题输出“更名方案 + 索引回写 + 引用替换” | cc | 2026-03-10 | done |
| A3-004 | 执行更名与引用替换并回写 `审计轮次: 3` | cc | 2026-03-12 | done |
| A3-005 | 生成 `S_round003` 清单并完成复审结论 | cc | 2026-03-12 | done |

## 特殊情况备注（仅在无需更名时填写）
| 编号 | 原因 | 风险 | 临时缓解 | 复审日期 | 评审人 |
| --- | --- | --- | --- | --- | --- |

## 复审结果
- 复审时间：2026-03-06 11:10 CST
- 复审结论：ROUND-003 已完成；`S_round003`（773 份）覆盖文档均已回写 `审计轮次: 3`，命名问题 `I3-001~I3-011` 全部完成更名与索引/引用回写，未留待办项。
- 当前进展：ROUND-003 收口完成，后续如开启新轮次按新轮次台账推进。
