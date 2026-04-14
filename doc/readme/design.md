# readme 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/readme/prd.md`
- 对应项目管理文档: `doc/readme/project.md`
- 对应文件级索引: `doc/readme/prd.index.md`

## 1. 设计定位
`readme` 模块的 `design.md` 负责描述对外入口文档、能力总览与产品口径的一致性设计。

## 2. 阅读顺序
1. `doc/readme/prd.md`
2. `doc/readme/design.md`
3. `doc/readme/project.md`
4. `doc/readme/prd.index.md`
5. 下钻 `gap/`、`governance/`、`production/` 等专题

## 3. 设计结构
- 入口层：README 与总览文档如何承接模块导航。
- 口径层：对外叙事、产品能力与限制如何统一。
- 闭环层：readme 与 site / engineering / game 的同步关系。

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
- 执行入口：`doc/readme/project.md`
- 兼容执行入口：`doc/readme/project.md`
- 索引入口：`doc/readme/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy `*.project.md` 长期保留，执行入口会继续双轨并存。
