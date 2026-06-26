# scripts 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/scripts/prd.md`
- 对应项目管理文档: `doc/scripts/project.md`
- 对应文件级索引: `doc/scripts/prd.index.md`

## 1. 设计定位
`scripts` 模块的 `design.md` 负责描述自动化脚本、precommit、链路脚本和治理脚本的组织设计。

## 2. 阅读顺序
1. `doc/scripts/prd.md`
2. `doc/scripts/design.md`
3. `doc/scripts/project.md`
4. `doc/scripts/prd.index.md`
5. 下钻 `precommit/`、`launcher/`、`wasm/` 等专题目录

## 3. 设计结构
- 开发前置层：precommit 与本地治理脚本。
- 运行支撑层：launcher / runtime / wasm 相关脚本。
- 文档治理层：doc-governance 与辅助脚本的组织方式。

## 4. 集成点
- `doc/engineering/prd.md`
- `scripts/doc-governance-check.sh`
- `testing-manual.md`

## 5. 专题导航
- precommit 类规则进入 `precommit/`
- launcher / chain / wasm 等专题保持同目录同 basename 管理

## 设计目标
- 提供 `scripts` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/scripts/prd.md`
- 执行入口：`doc/scripts/project.md`
- 索引入口：`doc/scripts/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
