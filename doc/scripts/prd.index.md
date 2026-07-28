# scripts PRD 文件级索引

审计轮次: 6

更新时间：2026-04-14

## 入口
- 模块 PRD：`doc/scripts/prd.md`
- 模块设计总览：`doc/scripts/design.md`
- 模块标准执行入口：`doc/scripts/project.md`
- governance 归并说明：`doc/scripts/governance/README.md`
- pre-commit 专题路由：`doc/scripts/precommit/README.md`
- WASM 历史专题路由：`doc/scripts/wasm/README.md`

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/scripts/precommit/pre-commit.prd.md` | 操作契约已收口于 PRD，不再维护独立 design | `doc/scripts/precommit/pre-commit.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 通用脚本入口、参数、worktree harness、bootstrap、PR closure 与 local landing 的稳定语义已归入 `doc/scripts/{prd,design,project}.md`；不再为已吸收的 dated triplet 保留索引。
- 默认文档配对为 `*.prd.md`、`*.design.md` 与同名 `*.project.md`；表内显式标记收口的专题不再为重复内容保留空壳 design。
- 首次选择 pre-commit 的当前门禁契约或失败修复流程时，先读
  `doc/scripts/precommit/README.md`；本页保留精确文件检索。
- 首次进入 WASM 专题时，先读 `doc/scripts/wasm/README.md`；它会把 absorbed historical
  build-std 记录与 world-runtime 的发布级 canonical pipeline 分开。历史记录位于
  `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md`。
