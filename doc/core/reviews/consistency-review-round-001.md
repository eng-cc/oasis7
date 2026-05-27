# Core 一致性审查记录（第001轮，草案）

审计轮次: 4

## 目的
- 为 `TASK-CORE-005` 提供轮次审查记录模板。
- 本轮主线-1：核对“当前文档”与“代码实现”是否对齐，偏差按规则登记并修复。
- 本轮主线-2：识别重复或过时文档，尽量归档并维护替代链/引用可达。

## 轮次信息
- 轮次编号: `ROUND-001`
- 轮次状态: `completed` (`not_started` | `in_progress` | `completed`)
- 审查时间窗: 2026-03-04 ~ 2026-03-05
- 审查负责人: cc

状态判定：
- `not_started`: 未开始抽样，且“一致性问题/整改项/复审结果”尚未形成有效记录。
- `in_progress`: 已启动抽样或登记问题，但尚未形成最终复审结论。
- `completed`: 已完成复审，阻断项关闭或已登记批准延期备注。

## 文档级审计标记方法（缺省=0）
- 每个受审文档采用字段 `审计轮次: <整数>` 标识最新已完成审计轮次。
- 规则：文档缺失该字段时，按 `审计轮次: 0` 处理（即未完成任何轮次审计）。
- 本轮目标：`ROUND-001` 收口时，纳入范围文档应满足 `审计轮次 >= 1`。
- 标记建议写在文档头部固定位置，便于检索与批量统计。
- 执行规则（即时回写）：
  - 当单篇文档完成“审计阅读”（无论是否改正文）时，必须在同一提交中回写该文档 `审计轮次: 1`。
  - 文档是否需要整改，与 `审计轮次` 标记解耦：已审读但存在问题的文档仍标记 `1`，问题通过“一致性问题/整改项”跟踪关闭。
  - 仅在“尚未完成本轮审读”时保持 `审计轮次: 0`（含字段缺失默认值）。
  - 若文档不适用本轮主线，需在“特殊情况备注”登记理由后方可不回写轮次。
- `轮次状态` 与标记关系：
  - `not_started`: 范围文档大多仍为缺省/0。
  - `in_progress`: 部分文档已更新到 `审计轮次: 1`，但仍有缺省/0。
  - `completed`: 范围文档全部达到 `审计轮次 >= 1`，且复审结论已落档。

建议统计命令：
```bash
rg -n "审计轮次:[[:space:]]*[1-9][0-9]*" doc/*/prd*.md doc/*/**/*.prd*.md
```

## 抽样范围
- 审计主线:
  - 代码对齐审计（文档条款 vs 实现行为）
  - 文档治理审计（重复/过时识别与归档）
- 分片执行：
  - A: `world-runtime` + `headless-runtime`
  - B: `world-simulator`
  - C: `p2p`
  - D: `engineering` + `testing` + `scripts`
  - E: `game` + `readme` + `site` + `playability_test_result`
- 模块清单：
  - `world-runtime`
  - `headless-runtime`
  - `world-simulator`
  - `p2p`
  - `game`
  - `readme`
  - `site`
  - `playability_test_result`
  - `testing`
  - `scripts`
  - `engineering`（方法口径 + 清单状态）
- 抽样文档（代表样本）：
  - `doc/world-runtime/prd.md`
  - `doc/world-runtime/governance/governance-events.md`
  - `doc/world-runtime/governance/audit-export.md`
  - `doc/world-runtime/runtime/runtime-integration.md`
  - `doc/world-runtime/wasm/wasm-interface.md`
  - `doc/headless-runtime/prd.md`
  - `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md`
  - `doc/p2p/distributed/distributed-runtime.prd.md`
  - `doc/p2p/distributed/distributed-hard-split-phase7.prd.md`
  - `doc/scripts/precommit/pre-commit.prd.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
  - `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.prd.md`
  - `doc/playability_test_result/game-test.prd.md`
  - `doc/readme/governance/readme-resource-model-layering.prd.md`
  - `doc/game/gameplay/gameplay-engineering-architecture.md`
- 抽样时间：2026-03-04 21:25 ~ 21:56 CST（并行分片）

## 受审文件清单（S_round001）
- 清单文件：`doc/core/reviews/round-001-reviewed-files.md`
- 生成规则：`rg -l "^审计轮次:\\s*1$" doc --glob '*.md' | sort`
- 当前基线（2026-03-05 00:33 CST）：`67` 份文档
- 用途：作为 `A-013` 的统计分母（仅对纳入本轮清单的文档判定“已审读/未审读”）。
- 验收命令：
  - `test -f doc/core/reviews/round-001-reviewed-files.md`
  - `rg -n "^审计轮次:\\s*1$" doc --glob '*.md' | wc -l`
  - `./scripts/doc-governance-check.sh`

## 一致性问题
| 编号 | 问题描述 | 影响范围 | 严重度 |
| --- | --- | --- | --- |
| I-001 | agent-browser wrapper 路径口径漂移：文档/脚本混用 `scripts/agent-browser`、`$REPO_ROOT/.codex/...`、`./.codex/...`，与当前执行口径不一致。 | `world-simulator/viewer`、`testing/manual`、`playability_test_result`、`site/doc`、`scripts/*` | high |
| I-002 | 多份活跃文档仍引用旧文档路径（如 `scripts/viewer-tools/*.prd.md`、`scripts/precommit/*.prd.md`），当前有效入口应为 `doc/scripts/...`。 | `engineering`、`testing/ci`、`world-simulator/viewer`、`p2p/node` | high |
| I-003 | `pre-commit` 文档对 `required` 套件定义与 `scripts/ci-tests.sh` 实现不一致（文档仍写 `sync-m1/m4/m5 --check`）。 | `doc/scripts/precommit/*`、`doc/testing/ci/*` | high |
| I-004 | launcher 生命周期文档引用不存在的测试文件 `crates/oasis7/src/bin/oasis7_game_launcher/tests.rs`。 | `doc/testing/launcher/*` | high |
| I-005 | world-runtime 治理/审计事件模型与代码不一致（`ShadowReport` 字段、事件枚举、失败类型）。 | `doc/world-runtime/governance/*`、`crates/oasis7/src/runtime/*` | high |
| I-006 | `audit-export.md` 的导出 API 描述（`limit/cursor` 分页）与实现 `save_audit_log(path, filter)` 不一致。 | `doc/world-runtime/governance/audit-export.md` | high |
| I-007 | world-runtime 根 PRD 生命周期状态机仍写 `deprecated/blocked`，与当前 ABI action 集合不一致。 | `doc/world-runtime/prd.md` | high |
| I-008 | world-simulator 旧 launcher 文档仍描述 native 直拉 game/chain；当前实现已切换到 `oasis7_web_launcher` 控制面。 | `doc/world-simulator/launcher/*` | medium |
| I-009 | web 控制台 PRD 状态机（`starting/stopping`）与当前 `ProcessState` 枚举不一致。 | `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md` | medium |
| I-010 | viewer 行数收口文档声称“全部 <=1200”，但当前多个 viewer 源文件已超限。 | `doc/world-simulator/viewer/*` | medium |
| I-011 | p2p 分布式文档仍含旧路径 `distributed_membership_sync/logic.rs`，phase7 验收主要锚定“旧路径删除”，缺少现状正向锚点。 | `doc/p2p/distributed/*` | medium |
| I-012 | site 中 4 处 gameplay top-level design 外链指向不存在的 `.md`（应为 `.prd.md`），产生 404。 | `site/index.html`、`site/en/index.html`、`site/doc/cn/index.html`、`site/doc/en/index.html` | high |
| I-013 | `game-test` 文档启动链路仍写 `oasis7_viewer_live + run-viewer-web.sh`，与现有 `oasis7_game_launcher` 流程不一致。 | `doc/playability_test_result/game-test.prd.md` | high |
| I-014 | 资源/计费口径在文档间不一致（`Compound/Hardware`、`Token Gas` 与实现 `Electricity/Data` 计费不一致）。 | `doc/readme/governance/*`、`doc/game/gameplay/*` | medium |
| I-015 | `release.md` 类型文档未纳入 `prd.index` 树，可追溯性不足。 | `doc/p2p/*/*.release.md` | medium |
| I-016 | `site/doc/*/viewer-manual.html` 外链仍指向历史 `.md` 路径（缺少 `/viewer/` 目录与 `.prd.md` 后缀），导致 GitHub 404。 | `site/doc/cn/viewer-manual.html`、`site/doc/en/viewer-manual.html` | high |

## 归档候选
| 编号 | 候选文档（簇） | 归档理由 | 替代入口 | 状态 |
| --- | --- | --- | --- | --- |
| R-001 | `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md`、`doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.prd.md` | 一次性设计对齐收口文档，项目状态已完成 | `doc/headless-runtime/prd.md` + `doc/core/reviews/consistency-review-round-001.md` | proposed |
| R-002 | `doc/world-runtime/runtime/runtime-numeric-correctness-phase1~15*.prd*.md`、`runtime-infinite-sequence-rollover.prd.md` | 阶段性里程碑文档大量保留在活跃区，维护噪声高 | `doc/world-runtime/prd.md`（建议补一份汇总页） | proposed |
| R-003 | legacy launcher desktop/unified 文档（已删除） | 已被 2026-03-04 控制面统一系列文档替代 | `game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md` | done |
| R-004 | legacy viewer rust line cap 文档（已删除） | 历史收口结论与当前代码状态漂移 | `viewer-release-full-coverage-gate.prd.md` + `doc/devlog/README.md` | done |
| R-005 | `doc/p2p/distributed/distributed-hard-split-phase7.prd*.md`、`doc/p2p/observer/observer-sync-*.prd*.md`（前序阶段） | 历史拆分/阶段文档已完成，信息可并入现行主线 | `doc/p2p/distributed/distributed-runtime.prd.md` + 最新 observer 桥接文档 | proposed |
| R-006 | `doc/p2p/*/*.release.md` | 与 PRD 树索引机制不一致，链路不可达 | 对应 `*.prd.md` + `*.project.md` 的状态段 | kept_with_index |
| R-007 | `doc/engineering/prd-review/checklists/*.md` | 旧快照法与“审计轮次 + ROUND 状态”方法重复，且存在结论漂移 | `doc/core/reviews/consistency-review-round-001.md` + 文档头 `审计轮次` | proposed |
| R-008 | `doc/site/github-pages/*.prd*.md`、`doc/site/manual/*.prd*.md` 中已完成且下一步为空的日期专题 | 大量已收口专题长期活跃化，降低主索引可读性 | `doc/site/prd.md`、`doc/site/project.md`、`site/doc/*/index.html` | proposed |
| R-009 | `doc/readme/gap|production|governance` 与 `doc/game/gameplay` 中“已完成+无后续”的日期专题 | 与主 PRD 重复，后续很少回写 | `doc/readme/prd.md`、`doc/game/prd.md`、`README.md` | proposed |

## 整改项
| 编号 | 整改动作 | 责任人 | 截止时间 | 状态 |
| --- | --- | --- | --- | --- |
| A-001 | 统一 agent-browser wrapper 口径（文档与脚本统一使用 `agent-browser`，并注明仓库开发副本 `.agents/...`） | testing + scripts + site 维护者 | 2026-03-07 | done |
| A-002 | 回写所有 `scripts/viewer-tools/*.prd.md`、`scripts/precommit/*.prd.md` 等旧路径为 `doc/scripts/...` | core + engineering 维护者 | 2026-03-07 | done |
| A-003 | 修正 launcher 生命周期专题测试路径（`oasis7_game_launcher/tests.rs` -> 当前真实测试文件）并补最小验收命令 | testing/launcher 维护者 | 2026-03-07 | done |
| A-004 | 修正 `pre-commit` required/full 套件描述，使其与 `scripts/ci-tests.sh` 当前行为一致 | scripts + testing/ci 维护者 | 2026-03-08 | done |
| A-005 | world-runtime 治理/审计口径对齐：`ShadowReport`、事件枚举、失败类型、导出 API 一次性统一（文档回写或代码补齐需二选一明确） | world-runtime 维护者 | 2026-03-10 | done |
| A-006 | world-runtime 基础语义对齐：生命周期状态机、`module_id/instance_id` 调度说明、Trap 映射、WASM 枚举（`Gameplay`/`Tick`） | world-runtime 维护者 | 2026-03-10 | done |
| A-007 | world-simulator launcher 文档收口：旧 desktop/unified 口径标记历史或归档，web-console 状态机回写到现状枚举 | world-simulator 维护者 | 2026-03-09 | done |
| A-008 | viewer 行数约束文档与现状对齐：更新“超限清单”并补持续审计命令 | world-simulator/viewer 维护者 | 2026-03-09 | done |
| A-009 | p2p 分布式文档对齐：修复旧路径引用，phase7 增加“当前四 crate 正向验收锚点”，并处理 `release.md` 可达性 | p2p 维护者 | 2026-03-09 | done |
| A-010 | 修复站点 404 链接与 game-test 启动链路描述，保证文档步骤与 `scripts/run-game-test*.sh` 一致 | site + playability 维护者 | 2026-03-07 | done |
| A-011 | 对齐资源/计费语义：`readme-resource-model-layering` 与 `gameplay-engineering-architecture` 统一为当前 runtime 实现口径 | readme + game 维护者 | 2026-03-09 | done |
| A-012 | 归档候选批次化执行：为 `R-001~R-009` 输出“保留文档 + 替代链 + 索引回写 + redirect”清单，并实施首批迁移（见 `doc/core/reviews/round-001-archive-migration-plan.md`） | core + 各模块维护者 | 2026-03-12 | done |
| A-013 | 对已完成本轮审读的文档回写 `审计轮次: 1`（缺省=0 保留），并以 `S_round001` 清单作为统计分母；执行要求为“单文档审计完成即同提交回写（与是否整改解耦）” | 各模块维护者 | 2026-03-12 | done |
| A-014 | 修复 `site/doc/*/viewer-manual.html` 对 GitHub 文档外链的路径漂移（统一 `/viewer/*.prd.md` 与 `capture-viewer-frame.prd.md`），并将该项加入 `site-manual-sync-check.sh` 阻断校验 | site + world-simulator/viewer 维护者 | 2026-03-07 | done |

## 整改验证快照（2026-03-05）
- A-002：`rg -n "scripts/(precommit|viewer-tools)/.*\\.prd(\\.project)?\\.md" doc --glob '*.md' | grep -v 'doc/scripts/'` 无命中。
- A-003：`test -f crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`；文档已补“最小验收命令”。
- A-004：`doc/scripts/precommit/pre-commit.prd.md` 已与 `scripts/ci-tests.sh` required/full 行为对齐并补验收命令。
- A-011：`readme-resource-model-layering` 与 `gameplay-engineering-architecture` 已统一为 `Electricity`/`Data` 计费口径。
- A-013：`S_round001` 已按 `审计轮次: 1` 规则重生成，统计基线更新为 67 份。

## 特殊情况备注（仅在无需整改时填写）
| 编号 | 原因 | 风险 | 临时缓解 | 复审日期 | 评审人 |
| --- | --- | --- | --- | --- | --- |
| S-001 | `runtime-numeric-correctness-phase*` 仍被部分历史链接/讨论引用，本批次先不立即归档 | 继续误导“当前状态”的风险 | 在 `doc/world-runtime/prd.index.md` 增加“历史里程碑”标记并指向主入口 | 2026-03-12 | cc |
| S-002 | `doc/engineering/prd-review/checklists/*` 仍承担 ROUND-001 执行期临时跟踪作用 | 与新方法并存造成重复维护 | ROUND-001 收口前只增量更新；收口后统一迁移或归档 | 2026-03-12 | cc |

## 复审结果
- 复审时间：2026-03-05 00:33 CST
- 复审结论：ROUND-001 两条主线均已完成。代码对齐整改项 `A-001~A-014` 全部关闭；文档治理侧已产出 `R-001~R-009` 批次化迁移清单并完成首批迁移（R-003/R-004），`R-002` 与 `R-007` 按 `S-001/S-002` 批准延期处理。轮次状态由 `in_progress` 收口为 `completed`。
