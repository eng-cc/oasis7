# core 审计与评审留痕

本目录保存 core 的历史审计、阶段评审与候选级放行证据；它不是当前规格或执行计划的首读入口。

## 先从哪里读

- 当前 core 范围、跨模块口径与正式决策：`doc/core/prd.md`
- 当前执行状态与下一步：`doc/core/project.md`
- 已知文件名的精确检索：`doc/core/prd.index.md`
- 需要追溯审计、评审或放行结论：按下方类型进入本目录。

## 留痕分流

| 需要回答的问题 | 入口 | 范围 |
| --- | --- | --- |
| 最近的文档入口与延期模块分流整改是什么 | `consistency-review-round-009.md`、`consistency-review-round-010.md` | ROUND-009/010 的主台账；其 `*-reviewed-files`、`*-kickoff-worklist` 与 `*-audit-progress-log` 是同轮支持证据 |
| 早期审计为何做出结构、职责、Design 补齐等决定 | `consistency-review-round-001.md` 至 `consistency-review-round-008.md` | 每轮主审计记录；同名 `round-*` worklist、reviewed-files 与 progress-log 仅作可复核附件 |
| 2026-03 版本候选的 readiness 或正式裁决 | `release-candidate-readiness-board-version-2026-03-11.md`、`release-candidate-go-no-go-version-2026-03-11.md` | 版本候选历史证据，不代表当前发布授权 |
| TASK-GAME-018 / TASK-CORE-005 / TASK-CORE-021 的收口依据 | `stage-closure-go-no-go-task-game-018-2026-03-10.md`、`task-core-005-round-audit-closure-2026-03-11.md`、`task-core-021-runtime-version-soak-closure-2026-03-11.md` | 专题或任务级历史收口记录 |

## 边界与保留规则

- 本目录的文件全部是审计证据或历史评审记录；即使已完成，也不能因不再是默认阅读面而删除。
- 当前规范、执行状态和新决策只更新 `doc/core/prd.md`、`doc/core/project.md` 及其现行专题，不回写旧 ROUND 记录。
- 若需要删除某份记录，必须先证明其已被现行 canonical 文档承接、没有仓库调用者、且不承担审计追溯职责；本轮核验未发现符合条件的文件。
