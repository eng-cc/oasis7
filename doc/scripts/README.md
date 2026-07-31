# scripts 文档入口

审计轮次: 11

本页只负责把读者路由到脚本模块的权威文档。脚本参数、工作流门禁和
任务完成状态不在这里复制，避免入口页与实现或规范分别演进。

## 按意图选择入口

| 目标 | 权威入口 |
| --- | --- |
| 理解脚本模块的职责、分层、接口与验收标准 | `doc/scripts/prd.md` |
| 理解模块结构和各文档层的关系 | `doc/scripts/design.md` |
| 查看脚本模块任务映射与完成记录 | `doc/scripts/prd.md` |
| 按文件名查找仍独立维护的专题文档 | `doc/scripts/prd.index.md` |
| 理解治理主题如何归入稳定模块权威 | `doc/scripts/governance/README.md` |
| 选择 pre-commit 当前契约或修复流程 | `doc/scripts/precommit/README.md` |
| 区分 WASM 历史追溯与当前发布级 pipeline | `doc/scripts/wasm/README.md` |
| 执行仓库任务生命周期 | `doc/engineering/workflow/source-of-truth.md` |
| 选择测试套件与本地验证路径 | `testing-manual.md` |

## 真值归属

- `doc/scripts/prd.md` 定义脚本能力的模块级需求和稳定契约，不承担任务状态。
- `doc/scripts/prd.md` 记录 PRD-ID 到任务和验证证据的映射，不重复规范正文。
- `doc/scripts/governance/README.md` 只解释治理主题的稳定权威归属；当前能力与兼容边界归入 `doc/scripts/{prd,design}.md`，历史任务归入 `doc/scripts/prd.md`。
- `doc/engineering/workflow/source-of-truth.md` 是工程任务生命周期、状态和门禁的唯一规范。脚本模块文档只描述相关 helper 的能力，不另建一套流程。
- 脚本自身的 `--help` 和测试是参数及机器可读输出的当前实现证据；入口页不固化易漂移的参数清单、端口或内部步骤。

## 文档树约束

- 模块根目录只保留 `README.md`、`prd.md`、`design.md`、GitHub task issue evidence comments 和 `prd.index.md`。
- 新专题按 `governance/`、`precommit/`、`wasm/` 或既有匹配主题目录落位；不要在模块根目录新增专题文件。
- 新增、移动或退役仍需独立维护的专题时，同步更新对应专题 README 与 `doc/scripts/prd.index.md`。
- 历史完成记录留在 GitHub task issue evidence comments 或 evidence 文档；已被当前规范吸收的一次性交接说明不再作为 live 导航入口。
