# engineering 文档索引

## 入口
- PRD: `doc/engineering/prd.md`
- 设计总览: `doc/engineering/design.md`
- 标准执行入口: `doc/engineering/project.md`
- 文件级索引: `doc/engineering/prd.index.md`

## 从这里开始
- 想看工程治理边界、验收条件与长期规则：`doc/engineering/prd.md`
- 想看当前治理窗口、活跃 follow-up 与 GitHub task issue / `task_uid` 追溯：`doc/engineering/project.md`
- 想按专题进入具体治理文档：`doc/engineering/prd.index.md`

## 模块职责
- 维护工程治理规则、文档组织标准与执行门禁。
- 跟踪文档迁移、文件级索引可达性与角色协作规范。
- 承接 engineering 治理趋势、季度审查与模板化流程沉淀。

## 按主题进入
- 文档治理、入口减重、存量维护成本、目录职责与 redirect 规则：统一从 `doc/engineering/doc-governance/README.md` 分流
- 环境分层、云上清单、仓库健康巡检与季度复核：统一从 `doc/engineering/governance/README.md` 分流
- Rust 体量治理、结构切片约束与 required gate：统一从 `doc/engineering/rust-governance/README.md` 分流
- `.pm` / self-evolution：当前 task truth 与 evidence sink 先看 `doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract`；repo-local memory / working_memory / stage-gate 对象背景再看 `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`，历史需求锚点保留为 `PRD-ENGINEERING-021`
- 历史迁移、审读记录与文件级专题检索：从 `doc/engineering/prd.index.md` 下钻；当前运行型工程治理资料从 `doc/engineering/governance/README.md` 下钻

## 共享约定
- 模块根入口、专题落位、README 职责与 legacy redirect 的共享治理规则统一从 `doc/engineering/doc-governance/README.md` 进入，再按问题下钻到规范正文或对应专题。
- 共享规则与专题长表统一回收到 `doc/engineering/prd.index.md` 与各专题 `*.project.md`，本页只保留 landing 所需分流。
