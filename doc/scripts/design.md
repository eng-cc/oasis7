# scripts 模块设计总览

审计轮次: 7

## 设计定位

本页描述 scripts 文档树的稳定抽象和阅读关系，不复制专题参数契约、
任务状态或工程 workflow 规则。

## 文档分层

| 层级 | 文档 | 职责 |
| --- | --- | --- |
| landing | `doc/scripts/README.md` | 按读者意图选择权威入口 |
| requirements | `doc/scripts/prd.md` | 定义模块边界、能力需求和验收标准 |
| architecture | `doc/scripts/design.md` | 解释模块结构与文档分层 |
| execution record | `doc/scripts/project.md` | 记录 PRD-ID、任务和验证证据映射 |
| inventory | `doc/scripts/prd.index.md` | 提供专题三件套的精确文件索引 |
| topic truth | `governance/`、`precommit/`、`wasm/` | 承载各主题的当前规范、设计和完成记录 |

工程任务生命周期不属于 scripts 模块的第二套设计层；它统一引用
`doc/engineering/workflow/source-of-truth.md`。

## 能力结构

- 开发与验证入口：为本地开发、检查和测试提供稳定 wrapper，并把正式验收边界交给测试规范。
- 任务与仓库治理 helper：实现 workflow 规范定义的机械操作，但不自行定义生命周期状态或门禁。
- 运行支撑入口：组合 launcher、runtime、provider 与 WASM 工具；具体参数和兼容边界由对应专题或脚本 `--help` 持有。
- 文档治理入口：通过模块索引和治理检查保持脚本、规范与验证证据可追溯。

## 阅读顺序

1. 从 `doc/scripts/README.md` 按目标选择入口。
2. 需要模块契约时读 `doc/scripts/prd.md`；需要当前任务证据时读 `doc/scripts/project.md`。
3. 需要某个治理问题时先进入对应专题 README，再读该专题的 PRD/design/project。
4. 已知文件名时使用 `doc/scripts/prd.index.md` 精确定位。

## 集成边界

- 工程生命周期：`doc/engineering/workflow/source-of-truth.md`
- 测试策略与 suite 选择：`testing-manual.md`
- 文档结构门禁：`scripts/doc-governance-check.sh`
- 跨模块工程需求：`doc/engineering/prd.md`

新增能力时先确定它属于模块级契约还是专题契约；只有跨多个专题的稳定
结构才回写本页，参数、端口、临时兼容路径和任务完成明细留在各自 owner 文档。
