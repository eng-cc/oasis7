# core 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/core/prd.md`
- 可变项目管理真值: GitHub task issue evidence comments
- 对应文件级索引: `doc/core/prd.index.md`

## 1. 设计定位
`core` 模块的 `design.md` 承担项目级“设计总入口”职责：
- 说明全仓文档与模块设计如何组织；
- 给出跨模块评审与治理的阅读顺序；
- 指明哪些内容继续下钻到各模块，不在 core 重复展开。

本文件不重写模块需求，也不替代各模块实现设计。

## 2. 阅读顺序
1. `doc/core/prd.md`：项目目标、模块地图、关键链路与治理基线。
2. `doc/core/design.md`：跨模块设计结构、入口分工、主链路导航。
3. GitHub task issue evidence comments：任务、依赖、状态与治理轮次。
4. `doc/core/prd.index.md`：核心专题索引与后续扩展入口。
5. 下钻到目标模块：`doc/<module>/README.md`、`prd.md` 与 `design.md`；任务状态从对应 GitHub task issue evidence comments 读取。

## 3. 设计结构
### 3.1 分层角色
- `doc/README.md`：仓库文档总导航。
- `doc/core/prd.md`：项目级 Why / What / Done。
- `doc/core/design.md`：项目级 How / Structure / Contract。
- GitHub task issue evidence comments：项目级可变 How / When / Who。
- `doc/<module>/*`：模块内具体设计与专题落地。

### 3.2 core 的设计边界
- 在 core 固化：模块地图、统一术语、跨模块链路、治理规则。
- 不在 core 固化：模块内部组件实现、专题级 API 细节、运行手册步骤。
- 若某个问题需要进入具体实现层，必须继续下钻到对应模块 `design.md` 或专题 `*.design.md`。

### 3.3 跨模块主链路
- 设计链路：`core -> <module>/prd.md -> <module>/design.md -> <topic>.design.md`
- 执行链路：`GitHub Project task -> GitHub task issue evidence comments -> 专业 PRD/design/evidence`
- 治理链路：`engineering/doc-structure-standard -> core ROUND 台账 -> 模块/专题改造`

## 4. 集成点
- `doc/README.md`
- `doc/core/prd.md`
- `doc/core/prd.index.md`
- `doc/engineering/doc-governance/doc-structure-standard.prd.md`
- 各模块入口：`doc/<module>/README.md`、`doc/<module>/prd.md`、`doc/<module>/design.md`

## 5. ROUND-006 入口职责
- ROUND 审计历史由 Git history 和 GitHub task issue evidence comments 追溯；`core` 不维护可变任务台账。
- 文档结构治理的裁定依据来自 `doc/engineering/doc-governance/doc-structure-standard.prd.md`。
- 任何模块入口补齐、专题文档治理和索引回写，都必须在 GitHub task issue evidence 中可追踪。

## 设计目标
- 提供 `core` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/core/prd.md`
- 执行入口：GitHub task issue evidence comments
- 索引入口：`doc/core/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
