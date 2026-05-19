# engineering 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/engineering/prd.md`
- 对应项目管理文档: `doc/engineering/project.md`
- 对应文件级索引: `doc/engineering/prd.index.md`

## 1. 设计定位
`engineering` 模块的 `design.md` 承担工程治理的总体设计入口：
- 说明工程规则如何从规范文档落到门禁、索引、迁移与审读流程；
- 说明文档治理、角色协作与质量门之间的关系；
- 指明结构治理轮（如 ROUND-006）与工程规范专题的衔接方式。

## 2. 阅读顺序
1. `doc/engineering/prd.md`：工程治理目标、规则、验收与追踪矩阵。
2. `doc/engineering/design.md`：工程治理的结构设计、规则载体与执行链路。
3. `doc/engineering/project.md`：工程任务拆解、治理批次与状态。
4. `doc/engineering/prd.index.md`：活跃专题入口。
5. 下钻专题：`doc-governance/`、`rust-governance/`、`prd-review/`、`doc-migration/`、`self-evolution/` 等。

## 3. 设计结构
### 3.1 规则载体分层
- `prd.md`：定义工程规则 Why / What / Done。
- `design.md`：定义规则如何组织、如何落地、由哪些脚本与文档承接。
- `project.md`：定义治理任务、批次、owner 与验证口径。
- `doc-governance/*`：定义 `doc/` 文档树的组织规范、默认阅读面减重规则与早期文档治理收口。
- `rust-governance/*`：定义 Rust 体量治理、冻结基线与结构切片 burn-down 规则。
- `prd-review/*`：定义全量审读/治理台账与进度追踪。
- `self-evolution/*`：定义 repo-native `.pm` 工作流、自我进化 memory 补强，以及外部 agent workflow 借鉴的 adopted / rejected / deferred 治理边界。

### 3.2 工程治理执行链路
- 规范定义：`doc/engineering/doc-governance/*`、`doc/engineering/rust-governance/*`、`doc/engineering/governance/*`
- 项目追踪：`doc/engineering/project.md`、专题 `*.project.md`
- 过程记录：`.pm/tasks/task_<32hex>.execution.md`
- 静态校验：`scripts/doc-governance-check.sh`
- ROUND 台账：`doc/core/reviews/consistency-review-round-*.md`

### 3.3 文档结构治理链路
- 权威规则：`doc/engineering/doc-governance/doc-structure-standard.prd.md`
- 规范正文：`doc/engineering/doc-governance/doc-structure-standard.design.md`
- 执行挂靠：`doc/engineering/doc-governance/doc-structure-standard.project.md`
- 项目级执行台账：`doc/core/reviews/consistency-review-round-006.md`
- 逐文档执行面：`doc/core/reviews/round-006-reviewed-files.md`

## 4. 集成点
- `AGENTS.md`
- `.agents/roles/*.md`
- `.agents/roles/templates/*.md`
- `doc/engineering/doc-governance/doc-structure-standard.prd.md`
- `doc/engineering/doc-governance/doc-structure-standard.design.md`
- `doc/engineering/prd-review/*.md`
- `scripts/doc-governance-check.sh`
- `testing-manual.md`

## 5. ROUND-006 入口职责
- `engineering` 负责提供 ROUND-006 的规则定义与裁定标准。
- 模块/专题是否需要 `design.md`、是否需要重命名为 `*.project.md`、是否存在职责混写，均以本模块规范专题为准。
- 若 ROUND-006 执行中发现规范空白，应先回写本模块规范文档，再继续治理。

## 设计目标
- 提供 `engineering` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/engineering/prd.md`
- 执行入口：`doc/engineering/project.md`
- 兼容执行入口：`doc/engineering/project.md`
- 索引入口：`doc/engineering/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy `*.project.md` 长期保留，执行入口会继续双轨并存。
- 若治理专题继续回流到 `engineering` 根目录，根入口会重新退化为专题堆放区。
