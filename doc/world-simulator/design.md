# world-simulator 模块设计总览

审计轮次: 6

- 对应需求文档: `doc/world-simulator/prd.md`
- 对应项目管理文档: `doc/world-simulator/project.md`
- 对应文件级索引: `doc/world-simulator/prd.index.md`

## 1. 设计定位
`world-simulator` 模块的 `design.md` 负责描述世界模拟、Viewer、Launcher、LLM 与场景系统的总体设计入口。

## 2. 阅读顺序
1. `doc/world-simulator/prd.md`
2. `doc/world-simulator/design.md`
3. `doc/world-simulator/project.md`
4. `doc/world-simulator/prd.index.md`
5. 下钻 `viewer/`、`launcher/`、`llm/`、`kernel/`、`scenario/`、`m4/` 等专题目录

## 3. 设计结构
- 交互层：Viewer / Launcher / Web Console / UI 流程。
- 模拟层：kernel、scenario、资源与世界状态演化。
- 智能层：LLM、agent 行为、间接控制与多场景评估。

## 4. 集成点
- `doc/world-runtime/prd.md`
- `doc/game/prd.md`
- `doc/site/prd.md`
- `doc/testing/prd.md`

## 5. 专题导航
- 交互体验进入 `viewer/`、`launcher/`
- 核心模拟进入 `kernel/`、`scenario/`
- LLM 与 agent 行为进入 `llm/`
- 发行阶段能力进入 `m4/`

## 设计目标
- 提供 `world-simulator` 模块的总体设计入口。

## 设计范围
- 覆盖模块级结构、主链路、分层与专题导航。
- 不替代专题 `*.design.md` 的细化设计。

## 关键接口 / 入口
- 需求入口：`doc/world-simulator/prd.md`
- 执行入口：`doc/world-simulator/project.md`
- 索引入口：`doc/world-simulator/prd.index.md`

## 设计演进计划
- M1 (2026-03-09): 在 ROUND-006 中补齐模块级 `design.md` 标准入口。
- M2: 按专题继续补齐高复杂度主题的 `*.design.md`。

## 设计风险
- 若专题级设计未及时补齐，模块级 `design.md` 可能承载过多导航职责。
- 若 legacy redirect 未明确标注为兼容跳转，读者可能误判历史入口为当前执行入口。
