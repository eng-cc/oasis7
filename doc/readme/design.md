# readme 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/readme/prd.md`
- 当前任务追踪: PR、实际 CI 与评审记录（Issue 按需）
- 对应文件级索引: `doc/readme/prd.index.md`

## 1. 设计定位
`readme` 模块的 `design.md` 负责描述对外入口文档、能力总览与产品口径的一致性设计。

## 2. 阅读顺序
1. `doc/readme/prd.md`
2. `doc/readme/design.md`
3. `doc/readme/prd.index.md`
5. 下钻 `gap/`、`governance/`、`production/` 等专题

## 3. 设计结构

### 组件职责与分层

- 入口层：README 与总览文档如何承接模块导航。
- 口径层：对外叙事、产品能力与限制如何统一。
- 闭环层：readme 与 site / engineering / game 的同步关系。

### 入口与口径同步主流程

[模块 PRD](prd.md) §2 的 Flow-RM-001/002/003定义两条相连链路：读者从 README 跳转模块入口并定位目标能力；维护者检测口径变更后更新入口、校验链接并同步发布，发布前巡检发现的冲突需修复后复核。[集成边界与异常处理](prd.md#4-technical-specifications)要求 README、模块 PRD 与 site 等入口保持一致；断链进入修复队列，并发编辑合并后重跑链接检查，legacy redirect 保留指向并声明主入口。

### 文档数据流与状态边界

- [模块 PRD](prd.md) §2 的功能矩阵定义入口导航的数据为模块名称、入口链接与摘要，内容状态为 `draft -> published -> refreshed`；口径同步的数据为术语、架构描述与更新时间，状态为 `detected -> synced -> verified`，由文档 owner 审核生效；链接巡检输出链接地址、状态和修复建议，状态为 `checked -> broken/fixed`。
- 这些是文档内容与检查结果的语义，不新增工程任务生命周期。当前交付事实仍从实际 PR、CI 与评审记录确认；存在口径冲突时，按[异常处理边界](prd.md#4-technical-specifications)禁止发布“已同步”状态。本页不以规划状态推断内容已经发布或巡检已经通过。

## 4. 集成点
- `doc/README.md`
- `doc/site/prd.md`
- `doc/engineering/doc-governance/doc-structure-standard.prd.md`

## 5. 专题导航
- 口径差距进入 `gap/`
- 规则整理进入 `governance/`
- 生产化闭环进入 `production/`

## 设计目标
- 提供 `readme` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/readme/prd.md`
- 执行入口：PR、实际 CI 与评审记录（Issue 按需）
- 索引入口：`doc/readme/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
