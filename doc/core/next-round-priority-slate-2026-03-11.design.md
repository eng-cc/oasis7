# oasis7: 下一轮跨模块优先级清单（2026-03-11）设计

- 对应需求文档: `doc/core/next-round-priority-slate-2026-03-11.prd.md`
- 对应项目管理文档: `doc/core/next-round-priority-slate-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
定义新一轮跨模块优先级排序结构，把“候选池 -> 排序 -> 第一优先级 -> handoff”固化为 core 层正式入口。

## 2. 设计结构
- 候选层：P0/P1/P2 候选主题与排序依据。
- 选择层：第一优先级主题、进入条件与理由。
- 交接层：owner role、handoff 输入输出与下一步执行入口。

## 3. 关键接口 / 入口
- `doc/core/next-round-priority-slate-2026-03-11.prd.md`
- `doc/core/next-round-priority-slate-2026-03-11.project.md`
- `doc/core/project.md`
- 历史输入：2026-03-11 模块主项目收口总览已退役删除；当前入口以本专题三件套、`doc/core/project.md`、`doc/core/prd.index.md` 与 GitHub task issue evidence comments 为准。

## 4. 约束与边界
- 本专题只做排序与入口冻结，不直接实现第一优先级功能。
- 第一优先级必须服务最短发布闭环。
- 已 completed 模块主项目不重新打开。

## 5. 设计演进计划
- 先冻结优先级清单。
- 再把第一优先级拆成正式专题。
- 后续按周期复盘是否需要调整排序。
