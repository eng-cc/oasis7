# Core 文档设计质量审查记录（第004轮）

审计轮次: 4

## 目的
- 为 `TASK-CORE-005` 提供 ROUND-004 审查入口，聚焦“文档设计质量与可维护性”。
- 本轮主线-1：检查文档体系的信息架构、分工边界、追溯闭环与可执行性。
- 本轮主线-2：识别设计层面的结构性缺陷，输出整改方案并完成回写。

## 轮次信息
- 轮次编号: `ROUND-004`
- 轮次状态: `completed` (`not_started` | `in_progress` | `completed`)
- 审查时间窗: 2026-03-06 ~ 2026-03-06
- 审查负责人: cc

状态判定：
- `not_started`: 仅完成启动文档与方法准备，尚未形成有效问题记录。
- `in_progress`: 已开始审读并登记问题/整改项，但未形成最终复审结论。
- `completed`: 设计问题已收口，整改项关闭或已登记批准延期备注。

## 文档级审计标记方法（缺省=0）
- 每个受审文档采用字段 `审计轮次: <整数>` 标识最新已完成审计轮次。
- 本轮执行规则：
  - 单篇文档完成 ROUND-004 审读后，在同一提交回写 `审计轮次: 4`（与是否需要整改解耦）。
  - 若判定“设计合格无需整改”，仍应回写 `审计轮次: 4`，并在本记录登记判定理由。
  - 若实施整改，需同步更新：`prd.md`/`prd.index.md`/`project.md` 与引用路径。
  - 若尚未完成 ROUND-004 审读，则保持原值（缺失按 `0`）。
- 本轮完成条件：纳入 `S_round004` 的文档全部满足 `审计轮次 >= 4`，且复审结论已落档。

### 即时回写机制（抗中断）
- 每读完 1 篇文档，必须立即完成两件事：
  - 回写该文档 `审计轮次: 4`。
  - 在审计进度日志登记 1 条结果记录（含时间、审计人、文档路径、结论、问题编号）。
- 严禁“先读一批、最后统一回写”；若中断，已登记日志应能准确反映已审读范围。
- 若仅发现问题但暂不整改，仍回写 `审计轮次: 4`，并在日志标记 `issue_open` 与 I4-* 编号。
- 若文档因冲突/权限暂无法回写，需先在日志标记 `blocked` 并记录原因，再进入下一个文档。

建议统计命令：
```bash
rg -n "^审计轮次:\s*4$" doc --glob '*.md'
```

## 重点审计维度（文档设计视角）
| 编号 | 维度 | 审计目标 | 严重度判定 |
| --- | --- | --- | --- |
| D4-001 | 信息架构 | `prd.md / prd.index.md / project.md` 入口层级清晰、无多入口冲突 | 入口冲突=high |
| D4-002 | 文档分工边界 | PRD 仅写 Why/What/Done，Project 仅写 How/When/Who，Devlog 仅写过程 | 分工串写=high |
| D4-003 | 可追溯闭环 | `PRD-ID -> TASK -> 验收命令 -> 测试证据` 全链可反查 | 断链=high |
| D4-004 | 可执行性 | 验收命令真实可跑、与正文描述一致、可复现 | 命令失效=high |
| D4-005 | 权威源与去重 | 同一规则/术语仅一处权威定义，其它文档只引用 | 双源漂移=high |
| D4-006 | 状态与时效 | 文档状态与更新时间真实，过时文档不误标 active | 状态失真=medium |
| D4-007 | 术语与规格一致性 | 核心术语、状态机、字段命名跨模块一致 | 语义冲突=medium |
| D4-008 | 发布可达性 | 站点链接、索引可达性、双语映射完整（无 404/孤儿文档） | 断链=high |

## 启动范围（设计风险分区）
- A: `doc/core/*` + `doc/engineering/*`（治理规则、模板与审计台账）
- B: `doc/world-simulator/*` + `doc/p2p/*`（专题密集区，追溯链复杂）
- C: `doc/testing/*` + `doc/scripts/*`（验收命令与门禁口径）
- D: `doc/site/*` + `doc/readme/*` + `doc/game/*`（外部入口与术语一致性）
- E: 其余模块与根入口（补查）

## 并行审计编组（6 子代理）
- G4-001 `Locke`：`doc/core/**` + `doc/engineering/**`
- G4-002 `Aristotle`：`doc/world-simulator/**`
- G4-003 `Beauvoir`：`doc/p2p/**`
- G4-004 `Socrates`：`doc/testing/**` + `doc/scripts/**` + `doc/playability_test_result/**`
- G4-005 `Feynman`：`doc/site/**` + `doc/readme/**` + `doc/game/**`
- G4-006 `Turing`：`doc/world-runtime/**` + `doc/headless-runtime/**` + 根入口补查
- 执行策略：分区并行审读，主代理统一汇总 I4-* 并去重后回写本台账。

## 受审文件清单（S_round004）
- 清单文件：`doc/core/reviews/round-004-reviewed-files.md`
- 生成规则：`rg -l "^审计轮次:\s*4$" doc --glob '*.md' | sort`
- 当前基线（2026-03-06 16:33 CST）：`788` 份文档（统计口径：`doc/**/*.md` 排除 `doc/devlog/**`）
- 用途：作为 ROUND-004 统计分母（仅对纳入本轮清单的文档判定“已审读/未审读”）。

## 审计进度日志（逐文档）
- 日志文件：`doc/core/reviews/round-004-audit-progress-log.md`
- 记录粒度：1 文档 1 记录（即时写入，不延后）。
- 字段：`时间`、`审计人/代理`、`文档路径`、`结论(pass/issue_open/blocked)`、`问题编号`、`备注`。

## 分区问题回收汇总（A4-006）
| 编组 | 审读范围 | 已审文件数 | 问题登记完成 | 主要问题编号（摘要） |
| --- | --- | --- | --- | --- |
| G4-001 | `doc/core/**` + `doc/engineering/**` | 45 | yes | I4-201/I4-202/I4-203 |
| G4-002 | `doc/world-simulator/**` | 314 | yes | I4-001/I4-002/I4-003/I4-004/I4-005/I4-006/I4-007/I4-008/I4-009/I4-010/I4-011 |
| G4-003 | `doc/p2p/**` | 147 | yes | I4-001/I4-003/I4-004/I4-005/I4-006/I4-008/I4-010/I4-011/I4-012/I4-013 |
| G4-004 | `doc/testing/**` + `doc/scripts/**` + `doc/playability_test_result/**` | 93 | yes | I4-001/I4-002/I4-003/I4-004/I4-005/I4-006/I4-008 |
| G4-005 | `doc/site/**` + `doc/readme/**` + `doc/game/**` | 102 | yes | I4-001/I4-002/I4-003/I4-004/I4-005/I4-006 |
| G4-006 | `doc/world-runtime/**` + `doc/headless-runtime/**` + 根入口补查 | 82 | yes | I4-001/I4-014/I4-015/I4-016/I4-017/I4-018/I4-019/I4-020/I4-021 |
| 合计 | 全分区 | 788 | yes | 已回收 6/6 子代理结果并完成纠偏补审 |

## 工作失误记录（2026-03-06）
| 编号 | 失误描述 | 发现时间 | 客观证据 | 影响 | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| E4-001 | 误将 `A4-002/A4-006` 提前标记为 `done`。根因是把 `S_round004` 的已打标集合当作审计分母，未与 `doc/**/*.md` 全量清单对账。 | 2026-03-06 14:47 CST | 失误时：`doc_total=820`、`doc_marked=491`、`doc_unmarked=329`；排除 `doc/devlog/**` 后 `297` 未打标，`doc/world-simulator/**` 为 `22/314`。纠偏后：`doc_no_devlog_marked=788/788`、`world-simulator=314/314`。 | 覆盖率判断一度失真；已通过纠偏恢复。 | closed |

## 弥补计划（纠偏）
| 编号 | 计划动作 | 负责人 | 截止时间 | 验收命令 | 状态 |
| --- | --- | --- | --- | --- | --- |
| F4-001 | 固化 ROUND-004 审计分母口径：`doc/**/*.md`（默认排除 `doc/devlog/**`），并在台账写明统计口径。 | cc | 2026-03-06 | `total=$(rg --files doc -g '*.md' -g '!doc/devlog/**' | wc -l); marked=$(rg -l '^审计轮次:\\s*4$' doc --glob '*.md' -g '!doc/devlog/**' | wc -l); test \"$marked\" -eq \"$total\"` | done |
| F4-002 | 补审 `doc/world-simulator/**` 剩余未覆盖文档并逐篇回写 `审计轮次: 4` 与审计日志。 | cc | 2026-03-07 | `total=$(rg --files doc/world-simulator -g '*.md' | wc -l); marked=$(rg -l '^审计轮次:\\s*4$' doc/world-simulator --glob '*.md' | wc -l); test \"$marked\" -eq \"$total\"` | done |
| F4-003 | 全量重生成 `S_round004` 并与审计日志做集合对账，确保“日志集合=打标集合”。 | cc | 2026-03-07 | `comm -3 <(awk -F'|' 'NR>4 && /2026-03-06/ {gsub(/`/,\"\",$4); gsub(/^ +| +$/,\"\",$4); if($4!=\"\" && $4!=\"文档路径\") print $4}' doc/core/reviews/round-004-audit-progress-log.md | sort -u) <(rg -l '^审计轮次:\\s*4$' doc --glob '*.md' | sort)` | done |
| F4-004 | 完成纠偏后再判定 `A4-002/A4-006` 与 `P1/B4-002` 是否可关闭，并回写复审说明。 | cc | 2026-03-08 | `rg -n '\\| A4-002 \\|.*\\| done \\||\\| A4-006 \\|.*\\| done \\|' doc/core/reviews/consistency-review-round-004.md` | done |

## 设计问题清单
| 编号 | 问题描述 | 影响范围 | 建议动作 | 严重度 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I4-001 | 索引/入口存在可达性断链或漏登记 | world-runtime/p2p/site/testing/world-simulator | 修复 `prd.index` 与入口互链，补齐孤儿文档登记 | high | closed（A4-010） |
| I4-002 | 验收命令口径不一致（如裸 `cargo check`） | site/testing/p2p/world-simulator | 统一仓库执行口径与命令模板 | high | closed（A4-009） |
| I4-003 | 追溯链字段缺失或审计字段重复导致口径冲突 | p2p/site/playability/core | 清理重复审计字段并补齐 PRD-ID 到任务链 | high | closed（A4-008/A4-011） |
| I4-004 | 文档含不可直接执行命令/占位命令 | p2p/testing/scripts/playability/site | 替换为仓内可执行命令并补示例 | high | closed（A4-009） |
| I4-005 | PRD-ID/任务编号体系不一致 | world-simulator/testing/site/p2p | 统一编号体系并回写映射 | high | closed（A4-008/A4-012） |
| I4-006 | 文档状态与最近更新时间失真 | p2p/site/playability/world-simulator | 回写真实状态与时间字段 | medium | deferred（ROUND-005） |
| I4-007 | PRD 混入 project/devlog 性质内容 | p2p/world-simulator | 清理串写，恢复 Why/What/Done 边界 | high | closed（A4-012） |
| I4-008 | 完成态缺日期/映射字段 | p2p/playability/world-simulator | 增加完成日期与映射字段 | medium | deferred（ROUND-005） |
| I4-009 | world-simulator 结构命名一致性偏差 | world-simulator | 统一目录/命名口径并回写索引 | medium | deferred（ROUND-005） |
| I4-010 | world-simulator 与 p2p 索引覆盖规则不一致 | world-simulator/p2p | 统一索引覆盖声明与入口规则 | medium | deferred（ROUND-005） |
| I4-011 | world-simulator 与 p2p 的追溯链声明不完整 | world-simulator/p2p | 补齐 PRD-ID 与任务追溯声明 | high | closed（A4-008） |
| I4-012 | p2p 主 PRD 缺可执行验收命令映射 | p2p | 在主 PRD 补充可执行验证映射 | high | closed（A4-008） |
| I4-013 | p2p 子项目 PRD-ID 粒度与主 PRD 不一致 | p2p | 统一 PRD-ID 粒度并校验引用 | high | closed（A4-008） |
| I4-014 | world-runtime 专题 traceability 链不闭合 | world-runtime | 补齐 PRD-ID->TASK->证据链 | high | closed（A4-008） |
| I4-015 | world-runtime project 任务缺 PRD-ID 映射与证据 | world-runtime | 为任务补齐映射与验收证据 | high | closed（A4-008） |
| I4-016 | world-runtime 旧模板文档未纳入索引且链路不全 | world-runtime | 升级模板并纳入索引 | high | closed（A4-004） |
| I4-017 | world-runtime/headless 主 PRD 验证仅描述性无命令 | world-runtime/headless-runtime | 增补可执行命令与证据路径 | high | closed（A4-008/A4-013） |
| I4-018 | world-runtime/headless project 任务缺验收命令 | world-runtime/headless-runtime | 为任务补充命令与证据链接 | high | closed（A4-008） |
| I4-019 | runtime 测试术语与治理事件口径不一致 | world-runtime | 统一术语并清理旧引用 | medium | closed（A4-013/A4-004） |
| I4-020 | `headless-runtime` 与 `nonviewer` 命名双轨 | headless-runtime | 收敛命名并补兼容说明 | medium | closed（A4-013） |
| I4-021 | root 补查入口的口径声明缺失 | 根入口 | 补充 root 入口到分区索引映射 | medium | closed（A4-011） |
| I4-201 | core/engineering project 追溯链条目不完整 | core/engineering | 补齐 PRD-ID->TASK->命令->证据 | high | closed（A4-008） |
| I4-202 | legacy 迁移快照存在可达性债务 | engineering/doc-migration | 制定迁移清理与豁免收敛计划 | medium | closed（A4-014） |
| I4-203 | ROUND-004 基线统计未与实时进度同步 | core/reviews | 统一 S_round004 统计与生成时间 | medium | closed（F4-001~F4-003） |

## high 严重度整改方案与影响面（A4-003）
| 编号 | 覆盖问题 | 整改方案（已执行） | 影响面 | 验收命令 |
| --- | --- | --- | --- | --- |
| H4-001 | I4-201/I4-014/I4-015/I4-017/I4-018 | 统一核心模块 `PRD-ID -> TASK -> 验收命令 -> 证据` 字段，消除 project 追溯断链。 | core/engineering/p2p/world-runtime/headless-runtime/testing | A4-008 |
| H4-002 | I4-002/I4-004 | 修正文档命令口径（含 precommit 占位命令与脚本路径），消除不可执行/占位命令。 | site/testing/scripts/playability | A4-009 |
| H4-003 | I4-001 | 修复站点与文档治理可达性问题并复跑治理检查。 | site/testing/scripts/playability/world-simulator | A4-010 |
| H4-004 | I4-003 | 清理重复行内审计字段，收敛审计口径与统计基线字段。 | core/reviews + 全文档集 | A4-011 |
| H4-005 | I4-005/I4-007 | 校验并清理 world-simulator PRD 串写，确保 PRD/project 分工与映射字段一致。 | world-simulator/viewer | A4-012 |
| H4-006 | I4-019/I4-020/I4-016 | 收敛 runtime 术语/路径口径并回写活跃文档引用（`world-runtime` 与 `headless-runtime`）。 | world-runtime/headless-runtime/p2p/game | A4-013 + A4-004 |
| H4-007 | I4-202 | 校验 legacy 迁移快照治理字段与可达性口径，确保迁移台账可审计。 | engineering/doc-migration | A4-014 |

## 整改项
| 编号 | 整改动作 | 责任人 | 截止时间 | 状态 |
| --- | --- | --- | --- | --- |
| A4-001 | 建立 ROUND-004 启动台账（本文件 + 审读清单 + 执行清单） | cc | 2026-03-06 | done |
| A4-002 | 完成 D4-001~D4-008 全量审读并登记问题清单 | cc | 2026-03-09 | done |
| A4-003 | 对 high 严重度问题输出“整改方案 + 影响面 + 验收命令” | cc | 2026-03-10 | done |
| A4-004 | 执行文档整改并回写审计轮次与索引引用 | cc | 2026-03-12 | done |
| A4-005 | 生成 `S_round004` 清单并完成复审结论 | cc | 2026-03-12 | done |
| A4-006 | 启动 6 子代理并行审计并回收分区问题清单 | cc | 2026-03-06 | done |
| A4-007 | 落实“逐文档即时回写”机制（审计轮次 + 进度日志）并纳入并行审计流程 | cc | 2026-03-06 | done |
| A4-008 | 验收命令：`rg -n "PRD-ID|验收命令|证据" doc/core/project.md doc/engineering/project.md doc/p2p/project.md doc/world-runtime/project.md doc/headless-runtime/project.md doc/testing/project.md`<br>`rg -n "PRD-ID.*TASK|TASK.*PRD-ID" doc/core/project.md doc/engineering/project.md doc/p2p/project.md doc/world-runtime/project.md doc/headless-runtime/project.md doc/testing/project.md` | cc | 2026-03-10 | done |
| A4-009 | 验收命令：`rg -n "env -u RUSTC_WRAPPER cargo check|\\./scripts/|bash scripts/" doc/site doc/testing doc/scripts doc/playability_test_result --glob '*.md'`<br>`! rg -n "\\$CODEX_HOME|<staged \\.rs files>|site/site/doc" doc/site doc/testing doc/scripts doc/playability_test_result --glob '*.md'` | cc | 2026-03-10 | done |
| A4-010 | 验收命令：`./scripts/doc-governance-check.sh`<br>`! rg -n "site/site/doc" doc/site doc/testing doc/scripts doc/playability_test_result --glob '*.md'`<br>`test -f doc/world-simulator/project.md` | cc | 2026-03-10 | done |
| A4-011 | 验收命令：`! rg -n "^- 审计轮次:\\s*[0-9]+" doc --glob '*.md'`<br>`rg -n "当前基线|当前已审读文档数" doc/core/reviews/consistency-review-round-004.md doc/core/reviews/round-004-reviewed-files.md` | cc | 2026-03-11 | done |
| A4-012 | 验收命令：`! rg -n "完成内容|遗留事项|时刻" doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.prd.md doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`<br>`rg -n "PRD-ID|任务拆解|验收命令" doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.project.md doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.project.md` | cc | 2026-03-11 | done |
| A4-013 | 验收命令：`! rg -n "doc/world-runtime\\.prd\\.md|ModuleValidationFailed" doc/world-runtime/testing/testing.md`<br>`rg -n "headless-runtime|nonviewer" doc/headless-runtime/README.md doc/headless-runtime/prd.md doc/headless-runtime/project.md` | cc | 2026-03-11 | done |
| A4-014 | 历史验收：2026-03-03 legacy migration backlog snapshot（后续已删除）曾用于检查 `legacy|快照|豁免|可达性` 语义；当前保留 `./scripts/doc-governance-check.sh` 作为可执行治理门禁。 | cc | 2026-03-12 | done |

## 特殊情况备注（仅在无需整改时填写）
| 编号 | 原因 | 风险 | 临时缓解 | 复审日期 | 评审人 |
| --- | --- | --- | --- | --- | --- |
| I4-006 | 该类问题为状态时效治理问题，需与下一轮“文档时效治理”联合执行，避免单轮局部改动造成新不一致。 | 文档更新时间与状态声明可能短期漂移。 | ROUND-004 仅保留结构/追溯/可执行性整改；时效性统一纳入 ROUND-005。 | 2026-03-15 | cc |
| I4-008 | 完成态字段补齐涉及跨模块回写窗口，需按模块 owner 批处理以减少反复改写。 | 个别历史文档缺完成日期字段。 | 在 ROUND-005 以模板化脚本+人工复核一次性补齐。 | 2026-03-15 | cc |
| I4-009 | world-simulator 命名一致性需与后续专题变更一并评估，避免短期二次更名。 | 局部命名不一致可能继续存在。 | ROUND-004 先固定追溯与口径，命名收敛延后至 ROUND-005。 | 2026-03-15 | cc |
| I4-010 | p2p/world-simulator 索引覆盖规则需与新增专题发布节奏联动，不适合本轮静态一次性回写。 | 索引声明可能出现边界差异。 | ROUND-005 引入统一索引覆盖模板后集中修订。 | 2026-03-15 | cc |

## 复审结果
- 复审时间：2026-03-06 16:33 CST
- 复审结论：ROUND-004 completed（high 问题已闭环；其余中风险项已登记延期并给出复审日期）。
- 当前进展：覆盖率口径失误 `E4-001` 已完成纠偏关闭（F4-001~F4-004 done）；`A4-001~A4-014` 全部完成；`S_round004` 维持 `788/788`（排除 `doc/devlog/**`）。
