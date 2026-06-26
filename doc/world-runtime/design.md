# world-runtime 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/world-runtime/prd.md`
- 对应项目管理文档: `doc/world-runtime/project.md`
- 对应文件级索引: `doc/world-runtime/prd.index.md`

## 1. 设计定位
`world-runtime` 模块的 `design.md` 负责描述世界内核、事件溯源、治理、WASM 执行与审计链路的总体设计。

## 2. 阅读顺序
1. `doc/world-runtime/prd.md`
2. `doc/world-runtime/design.md`
3. `doc/world-runtime/project.md`
4. `doc/world-runtime/prd.index.md`
5. 下钻 `governance/`、`module/`、`runtime/`、`wasm/`、`testing/` 等专题目录

## 3. 设计结构
- 内核层：状态、事件、tick、审计与回放。
- 执行层：WASM 模块、发布链路、治理审批。
- 验证层：runtime testing、shadow、回归与证据。

## 4. 集成点
- `doc/p2p/prd.md`
- `doc/world-simulator/prd.md`
- `doc/testing/prd.md`

## 5. 专题导航
- 核心治理进入 `governance/`
- 模块发布与实体进入 `module/`
- 运行时行为进入 `runtime/`
- 执行器与 ABI 进入 `wasm/`

## 设计目标
- 提供 `world-runtime` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/world-runtime/prd.md`
- 执行入口：`doc/world-runtime/project.md`
- 索引入口：`doc/world-runtime/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
